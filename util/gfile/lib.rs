use auth_client::AuthServer;
use std::path::Path;

const STORAGE_API: &'static str = "https://storage.googleapis.com/storage/v1/b";
const UPLOAD_API: &'static str = "https://storage.googleapis.com/upload/storage/v1/b";
const BUFFER_SIZE: usize = 1048576;

pub enum GFile {
    LocalFile(std::fs::File),
    RemoteFile(GoogleCloudFile),
}

#[derive(PartialEq)]
pub enum Mode {
    Read,
    Write,
}

pub struct GoogleCloudFile {
    bucket: String,
    object: String,
    token: Option<String>,
    resumable_url: Option<String>,
    buf: Vec<u8>,
    size: u64,
    index: u64,
    mode: Mode,
    buf_start_index: u64,
}

#[derive(Debug, PartialEq)]
pub enum GPath<'a> {
    LocalPath(&'a Path),
    RemotePath(&'a str, &'a str),
}

impl<'a> GPath<'a> {
    pub fn from_path(p: &'a Path) -> Self {
        if !p.starts_with("/cns/") {
            return GPath::LocalPath(p);
        }

        let path_str = p.to_str().unwrap();
        let mut split_path = path_str.split("/");
        split_path.next();
        split_path.next();
        let bucket = match split_path.next() {
            Some(b) => b,
            None => return GPath::LocalPath(p),
        };
        let mut object = &path_str[5 + bucket.len()..];
        if object.starts_with("/") {
            object = &object[1..];
        }

        GPath::<'a>::RemotePath(bucket, object)
    }
}

impl GFile {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<GFile> {
        match GPath::from_path(path.as_ref()) {
            GPath::LocalPath(_) => {
                let f = std::fs::File::open(path)?;
                Ok(GFile::LocalFile(f))
            }
            GPath::RemotePath(bucket, object) => {
                let token = if let Some(c) = auth_client::get_global_client() {
                    let response = c.get_gcp_token(c.token.clone());
                    Some(response.gcp_token.clone())
                } else {
                    None
                };
                let f = GoogleCloudFile::open(token, bucket, object)?;
                Ok(GFile::RemoteFile(f))
            }
        }
    }

    pub fn create<P: AsRef<Path>>(path: P) -> std::io::Result<GFile> {
        match GPath::from_path(path.as_ref()) {
            GPath::LocalPath(p) => {
                // Recursively recreate parent directories if they don't already exist.
                // This matches the behaviour of google cloud when you create a file
                match p.parent() {
                    Some(p) => std::fs::create_dir_all(p)?,
                    None => (),
                };

                let f = std::fs::File::create(path)?;
                Ok(GFile::LocalFile(f))
            }
            GPath::RemotePath(bucket, object) => {
                let c = auth_client::get_global_client().unwrap();
                let response = c.get_gcp_token(c.token.clone());
                let f = GoogleCloudFile::create(&response.gcp_token, bucket, object)?;
                Ok(GFile::RemoteFile(f))
            }
        }
    }

    pub fn read_dir<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<String>> {
        match GPath::from_path(path.as_ref()) {
            GPath::LocalPath(p) => {
                let mut output = Vec::new();
                let mut dirs = vec![p.to_owned()];
                let mut steps = 0;
                while let Some(dir) = dirs.pop() {
                    steps += 1;
                    for entry in std::fs::read_dir(dir.to_owned())? {
                        let entry = entry?;

                        if entry.path() == dir {
                            continue;
                        }

                        if entry.metadata()?.is_dir() {
                            dirs.push(entry.path().to_owned());
                        } else {
                            output.push(entry.path().to_str().unwrap().to_string());
                        }
                    }

                    // No idea if this is needed, but in case some kind of loop occurs
                    // by following symlinks, just quit early.
                    if steps > 1024 {
                        break;
                    }
                }

                Ok(output)
            }
            GPath::RemotePath(bucket, object) => {
                let token = if let Some(c) = auth_client::get_global_client() {
                    let response = c.get_gcp_token(c.token.clone());
                    Some(response.gcp_token)
                } else {
                    None
                };
                GoogleCloudFile::read_dir(token, bucket, object)
            }
        }
    }
}

impl GoogleCloudFile {
    fn open(token: Option<String>, bucket: &str, object: &str) -> std::io::Result<Self> {
        let mut req = hyper::Request::get(format!(
            "{}/{}/o/{}?alt=json",
            STORAGE_API,
            bucket,
            ws_utils::urlencode(&object)
        ));
        if let Some(token) = &token {
            req = req.header(hyper::header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let req = req.body(hyper::Body::from(String::new())).unwrap();

        let m = match requests::request(req) {
            Ok(m) => m,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "file does not exist (request failed)",
                ))
            }
        };

        if m.status_code != 200 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "file does not exist (status code {}, body {})",
                    m.status_code,
                    std::str::from_utf8(&m.body).unwrap()
                ),
            ));
        }

        let parsed = json::parse(std::str::from_utf8(&m.body).unwrap()).unwrap();
        let size: u64 = parsed["size"].as_str().unwrap().parse().unwrap();

        Ok(Self {
            token: token,
            bucket: bucket.to_string(),
            object: object.to_string(),
            resumable_url: None,
            buf: Vec::new(),
            size: size,
            index: 0,
            mode: Mode::Read,
            buf_start_index: 0,
        })
    }

    fn create(token: &str, bucket: &str, object: &str) -> std::io::Result<Self> {
        let content_type = ws_utils::content_type(object).unwrap_or("application/octet-stream");

        let req = hyper::Request::post(format!(
            "{}/{}/o?uploadType=resumable&name={}",
            UPLOAD_API, bucket, object
        ))
        .header(hyper::header::AUTHORIZATION, format!("Bearer {}", token))
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(hyper::Body::from(format!(
            r#"{{"contentType": "{}"}}"#,
            content_type
        )))
        .unwrap();

        let m = match requests::request(req) {
            Ok(m) => m,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "file does not exist",
                ))
            }
        };

        if m.status_code != 200 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "file does not exist",
            ));
        }

        let location;
        if let Some(l) = m.headers.get("Location") {
            location = l.to_str().unwrap().to_string()
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "file does not exist",
            ));
        }

        let mut s = Self {
            token: Some(token.to_string()),
            bucket: bucket.to_string(),
            object: object.to_string(),
            resumable_url: Some(location),
            buf: Vec::new(),
            size: 0,
            index: 0,
            mode: Mode::Write,
            buf_start_index: 0,
        };
        s.confirm_reupload_status()?;

        Ok(s)
    }

    fn refill_buffer(&mut self, index: u64) -> std::io::Result<()> {
        let start = if self.index + BUFFER_SIZE as u64 > self.size {
            let overhang: i64 = (self.index as i64) + (BUFFER_SIZE as i64) - self.size as i64;
            std::cmp::max(0, (index as i64) - overhang) as u64
        } else {
            index
        };

        let content_range = format!(
            "bytes={}-{}",
            start,
            std::cmp::min(self.index + BUFFER_SIZE as u64, self.size)
        );

        let mut req = hyper::Request::get(format!(
            "{}/{}/o/{}?alt=media",
            STORAGE_API,
            self.bucket,
            ws_utils::urlencode(&self.object)
        ));

        if let Some(token) = &self.token {
            req = req.header(hyper::header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let req = req
            .header(hyper::header::RANGE, content_range)
            .body(hyper::Body::from(String::new()))
            .unwrap();

        let m = match requests::request(req) {
            Ok(m) => m,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("file does not exist: {e:#?}"),
                ))
            }
        };

        if !requests::is_success(m.status_code) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("file does not exist: status code {}", m.status_code),
            ));
        }

        self.buf = m.body;
        self.index = index;
        self.buf_start_index = start;

        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        if self.buf.len() < BUFFER_SIZE {
            return Ok(buf.len());
        }

        self.flush(false)?;
        Ok(buf.len())
    }

    fn cancel(&mut self) -> std::io::Result<()> {
        let req = hyper::Request::delete(self.resumable_url.as_ref().unwrap())
            .header(
                hyper::header::AUTHORIZATION,
                format!(
                    "Bearer {}",
                    self.token.as_ref().map(|s| s.as_str()).unwrap_or("")
                ),
            )
            .header(hyper::header::CONTENT_LENGTH, "0")
            .body(hyper::Body::from(String::new()))
            .unwrap();

        requests::request(req)?;

        Ok(())
    }

    fn confirm_reupload_status(&mut self) -> std::io::Result<()> {
        if self.get_reupload_status() {
            return Ok(());
        }

        self.cancel().unwrap();

        let mut req = hyper::Request::post(format!(
            "{}/{}/o?uploadType=resumable&name={}",
            UPLOAD_API,
            self.bucket,
            ws_utils::urlencode(&self.object),
        ));

        if let Some(token) = &self.token {
            req = req.header(hyper::header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let req = req
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(hyper::Body::from("{}".to_string()))
            .unwrap();

        let response = requests::request(req)?;
        if let Some(l) = response.headers.get("Location") {
            self.resumable_url = Some(l.to_str().unwrap().to_string());
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no location provided",
            ));
        }

        Ok(())
    }

    fn get_reupload_status(&mut self) -> bool {
        let mut req = hyper::Request::put(self.resumable_url.as_ref().unwrap());

        if let Some(token) = &self.token {
            req = req.header(hyper::header::AUTHORIZATION, format!("Bearer {}", token));
        }

        let req = req
            .header(hyper::header::CONTENT_LENGTH, "0")
            .header(hyper::header::CONTENT_RANGE, "bytes */*")
            .body(hyper::Body::from(String::new()))
            .unwrap();

        let response = match requests::request(req) {
            Ok(r) => r,
            Err(_) => return false,
        };

        if response.status_code == 308 {
            if let Some(_) = response.headers.get("Range") {
                // Some data has already been uploaded, but we don't know what it was.
                // So let's cancel this upload and restart a new one.
                return false;
            }
            return true;
        }

        response.status_code == 200 || response.status_code == 201
    }

    fn flush(&mut self, finished: bool) -> std::io::Result<()> {
        let end_range = self.size + self.buf.len() as u64;
        if finished && end_range == 0 {
            return Ok(());
        }

        let content_range = if finished {
            format!("bytes {}-{}/{}", self.size, end_range - 1, end_range)
        } else {
            format!("bytes {}-{}/*", self.size, end_range - 1)
        };

        let req = hyper::Request::post(self.resumable_url.as_ref().unwrap())
            .header(
                hyper::header::AUTHORIZATION,
                format!(
                    "Bearer {}",
                    self.token.as_ref().map(|s| s.as_str()).unwrap_or("")
                ),
            )
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .header(hyper::header::CONTENT_RANGE, content_range.as_str())
            .header(hyper::header::CONTENT_LENGTH, self.buf.len())
            .body(hyper::Body::from(self.buf.clone()))
            .unwrap();

        let response = requests::request(req)?;
        if response.status_code != 200 && response.status_code != 308 && response.status_code != 201
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "bad status code during flush: {}, response: {}",
                    response.status_code,
                    String::from_utf8(response.body).unwrap()
                ),
            ));
        }

        let mut parsed_range_header = false;

        if let Some(range) = response.headers.get("Range") {
            let values: Vec<_> = range.to_str().unwrap().split("-").collect();
            if values.len() == 2 {
                if let Ok(offset) = values[1].parse::<u64>() {
                    let taken = (offset - self.size) as usize;
                    self.buf = self.buf.split_off(taken);
                    self.size = offset;
                    parsed_range_header = true;
                }
            }
        }

        if !parsed_range_header {
            self.size += self.buf.len() as u64;
            self.buf.clear();
        }

        self.buf.clear();

        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Fill in the write buffer from the cached data.
        let mut buf_index = 0;
        loop {
            if self.index == self.size {
                return Ok(buf_index);
            }

            while self.index >= self.buf_start_index
                && self.index - self.buf_start_index < self.buf.len() as u64
                && buf_index < buf.len()
            {
                buf[buf_index] = self.buf[(self.index - self.buf_start_index) as usize];
                self.index += 1;
                buf_index += 1;
            }

            // If we've filled the write buffer, quit.
            if buf_index == buf.len() {
                return Ok(buf.len());
            }

            if self.index == self.size {
                return Ok(buf_index);
            }

            // We haven't filled the write buffer and we've exhausted the cached data.
            // So let's request more data.
            self.refill_buffer(self.index)?;
        }
    }

    pub fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            std::io::SeekFrom::Start(pos) => pos as i64,
            std::io::SeekFrom::End(pos) => (self.size as i64) + pos,
            std::io::SeekFrom::Current(pos) => (self.index as i64) + pos,
        };

        self.index = std::cmp::max(0, std::cmp::min(self.size as i64, new_pos)) as u64;
        Ok(self.index)
    }

    pub fn read_dir(
        token: Option<String>,
        bucket: &str,
        prefix: &str,
    ) -> std::io::Result<Vec<String>> {
        let mut req =
            hyper::Request::get(format!("{}/{}/o?prefix={}", STORAGE_API, bucket, prefix));

        if let Some(token) = token {
            req = req.header(hyper::header::AUTHORIZATION, format!("Bearer {}", token))
        }

        let req = req.body(hyper::Body::from(String::new())).unwrap();

        let m = match requests::request(req) {
            Ok(m) => m,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "folder does not exist",
                ))
            }
        };

        let parsed = json::parse(std::str::from_utf8(&m.body).unwrap()).unwrap();
        let mut output = Vec::new();
        for item in parsed["items"].members() {
            output.push(format!(
                "/cns/{}/{}",
                bucket,
                item["name"].as_str().unwrap()
            ));
        }

        Ok(output)
    }
}

impl std::io::Write for GFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            GFile::LocalFile(f) => f.write(buf),
            GFile::RemoteFile(f) => f.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            GFile::LocalFile(f) => f.flush(),
            // We don't actually support flushing arbitrarily small buffers. Google
            // will just not allow it.
            GFile::RemoteFile(_) => Ok(()),
        }
    }
}

impl std::io::Seek for GFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            GFile::LocalFile(f) => f.seek(pos),
            GFile::RemoteFile(f) => f.seek(pos),
        }
    }
}

impl std::io::Read for GFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            GFile::LocalFile(f) => f.read(buf),
            GFile::RemoteFile(f) => f.read(buf),
        }
    }
}

impl Drop for GFile {
    fn drop(&mut self) {
        match self {
            GFile::RemoteFile(f) => {
                if f.mode == Mode::Write {
                    f.flush(true).unwrap()
                }
            }
            _ => return,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate primitive;
    extern crate recordio;

    use std::io::Read;

    #[test]
    fn test_get_path() {
        assert_eq!(
            GPath::from_path("/cns/iq-d/home/colinmerkel/tmp.txt".as_ref()),
            GPath::RemotePath("iq-d", "home/colinmerkel/tmp.txt"),
        );
        assert_eq!(
            GPath::from_path("/home/colinmerkel/tmp.txt".as_ref()),
            GPath::LocalPath("/home/colinmerkel/tmp.txt".as_ref()),
        );
    }

    //#[test]
    fn test_list_dirs() {
        assert_eq!(GFile::read_dir("/tmp/data").unwrap(), vec![String::new()]);
    }

    //#[test]
    fn test_get_token() {
        let access = String::from("abcdef");
        let client = auth_client::AuthClient::new("127.0.0.1", 8888);
        client.global_init(access);
        {
            let mut f = GFile::open("/cns/colossus/my_crazy_test.txt").unwrap();
            let mut buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            let result = f.read(&mut buf);
            assert_eq!(result.unwrap(), 7);
            assert_eq!(&buf, &[1, 2, 3, 4, 5, 6, 7, 0, 0, 0]);
        }
    }

    //#[test]
    fn test_write_recordio() {
        let access = String::from("abcdef");
        let client = auth_client::AuthClient::new("127.0.0.1", 8888);
        client.global_init(access);
        {
            let mut f = GFile::create("/cns/colossus/data.recordio").unwrap();
            let mut t = recordio::RecordIOWriter::new(&mut f);
            for _ in 0..500000 {
                t.write(&primitive::Primitive::from(0 as u64));
            }
        }
    }

    //#[test]
    fn test_read_recordio() {
        let access = String::from("abcdef");
        let client = auth_client::AuthClient::new("127.0.0.1", 8888);
        client.global_init(access);
        {
            let mut f = GFile::open("/cns/colossus/data.recordio").unwrap();
            let mut t =
                recordio::RecordIOReaderOwned::<primitive::Primitive<u64>>::new(Box::new(f));
            let output = t.collect::<Vec<_>>();
            assert_eq!(output, vec![primitive::Primitive::from(0 as u64)]);
        }
    }

    //#[test]
    fn test_list_directory() {
        let access = String::from("abcdef");
        let client = auth_client::AuthClient::new("127.0.0.1", 8888);
        client.global_init(access);
        {
            let mut list = GFile::read_dir("/cns/colossus").unwrap();
            assert_eq!(list, vec![String::from("/cns/colossus/data.recordio")]);
        }
    }

    //#[test]
    fn test_list_directory_2() {
        let access = String::from("abcdef");
        let client = auth_client::AuthClient::new("127.0.0.1", 8888);
        client.global_init(access);
        {
            let mut list = GFile::read_dir("/tmp/data").unwrap();
            assert_eq!(list, vec![String::from("/tmp/data/LargetableReadLog")]);
        }
    }
}

use auth_client::AuthServer;
use futures::future;
use futures::future::Future;
use futures::stream::Stream;
use std::fs::File;
use std::path::Path;

use std::io::Read;
use std::io::Seek;
use std::io::Write;

const STORAGE_API: &'static str = "https://storage.googleapis.com/storage/v1/b";
const UPLOAD_API: &'static str = "https://storage.googleapis.com/upload/storage/v1/b";
const BUCKET: &'static str = "colossus";
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
    token: String,
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
        let object = &path_str[6 + bucket.len()..];

        GPath::<'a>::RemotePath(bucket, object)
    }
}

impl GFile {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<GFile> {
        match GPath::from_path(path.as_ref()) {
            GPath::LocalPath(p) => {
                let f = std::fs::File::open(path)?;
                Ok(GFile::LocalFile(f))
            }
            GPath::RemotePath(bucket, object) => {
                let c = auth_client::get_global_client().unwrap();
                let response = c.get_gcp_token(c.token.clone());
                let f = GoogleCloudFile::open(response.get_gcp_token(), bucket, object)?;
                Ok(GFile::RemoteFile(f))
            }
        }
    }

    pub fn create<P: AsRef<Path>>(path: P) -> std::io::Result<GFile> {
        match GPath::from_path(path.as_ref()) {
            GPath::LocalPath(p) => {
                let f = std::fs::File::create(path)?;
                Ok(GFile::LocalFile(f))
            }
            GPath::RemotePath(bucket, object) => {
                let c = auth_client::get_global_client().unwrap();
                let response = c.get_gcp_token(c.token.clone());
                let f = GoogleCloudFile::create(response.get_gcp_token(), bucket, object)?;
                Ok(GFile::RemoteFile(f))
            }
        }
    }
}

impl GoogleCloudFile {
    fn open(token: &str, bucket: &str, object: &str) -> std::io::Result<Self> {
        let req = hyper::Request::get(format!("{}/{}/o/{}?alt=json", STORAGE_API, bucket, object))
            .header(hyper::header::AUTHORIZATION, format!("Bearer {}", token))
            .body(hyper::Body::from(String::new()))
            .unwrap();

        let https = hyper_tls::HttpsConnector::new(1).unwrap();
        let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);

        let f = Box::new(
            client
                .request(req)
                .map_err(|_| ())
                .and_then(|res| {
                    if res.status() == hyper::StatusCode::OK {
                        future::ok(res.into_body().concat2().map_err(|_| ()))
                    } else {
                        future::err(())
                    }
                })
                .and_then(|res| res)
                .and_then(move |response| {
                    let response = String::from_utf8(response.into_bytes().to_vec()).unwrap();
                    future::ok(response)
                }),
        );

        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let output = match runtime.block_on(f) {
            Ok(m) => m,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "file does not exist",
                ))
            }
        };

        let parsed = json::parse(&output).unwrap();
        let size: u64 = parsed["size"].as_str().unwrap().parse().unwrap();

        Ok(Self {
            token: token.to_string(),
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
        let req = hyper::Request::post(format!(
            "{}/{}/o?uploadType=resumable&name={}",
            UPLOAD_API, bucket, object
        ))
        .header(hyper::header::AUTHORIZATION, format!("Bearer {}", token))
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(hyper::Body::from("{}".to_string()))
        .unwrap();

        let https = hyper_tls::HttpsConnector::new(1).unwrap();
        let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);

        let f = Box::new(client.request(req).map_err(|e| ()).and_then(|res| {
            if res.status() == hyper::StatusCode::OK {
                if let Some(l) = res.headers().get("Location") {
                    return future::ok(l.to_str().unwrap().to_string());
                }
            }
            future::err(())
        }));

        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let output = match runtime.block_on(f) {
            Ok(m) => m,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "file does not exist",
                ))
            }
        };

        Ok(Self {
            token: token.to_string(),
            bucket: bucket.to_string(),
            object: object.to_string(),
            resumable_url: Some(output),
            buf: Vec::new(),
            size: 0,
            index: 0,
            mode: Mode::Write,
            buf_start_index: 0,
        })
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

        let req = hyper::Request::get(format!(
            "{}/{}/o/{}?alt=media",
            STORAGE_API, self.bucket, self.object
        ))
        .header(
            hyper::header::AUTHORIZATION,
            format!("Bearer {}", self.token),
        )
        .header(hyper::header::RANGE, content_range)
        .body(hyper::Body::from(String::new()))
        .unwrap();

        let https = hyper_tls::HttpsConnector::new(1).unwrap();
        let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);

        let f = Box::new(
            client
                .request(req)
                .map_err(|_| ())
                .and_then(|res| {
                    if res.status().is_success() {
                        future::ok(res.into_body().concat2().map_err(|_| ()))
                    } else {
                        future::err(())
                    }
                })
                .and_then(|res| res)
                .and_then(move |response| {
                    let response = response.into_bytes().to_vec();
                    future::ok(response)
                }),
        );

        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let output = match runtime.block_on(f) {
            Ok(m) => m,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "file does not exist",
                ))
            }
        };

        std::mem::replace(&mut self.buf, output);
        self.index = index;
        self.buf_start_index = start;

        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        if self.buf.len() < BUFFER_SIZE {
            return Ok(buf.len());
        }

        let bytes = self.buf.len();
        self.flush(false)?;
        Ok(bytes)
    }

    fn flush(&mut self, finished: bool) -> std::io::Result<()> {
        let end_range = self.size + self.buf.len() as u64;
        let content_range = if finished {
            format!("bytes {}-{}/{}", self.size, end_range - 1, end_range)
        } else {
            format!("bytes {}-{}/*", self.size, end_range - 1)
        };

        let req = hyper::Request::post(self.resumable_url.as_ref().unwrap())
            .header(
                hyper::header::AUTHORIZATION,
                format!("Bearer {}", self.token),
            )
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .header(hyper::header::CONTENT_RANGE, content_range.as_str())
            .body(hyper::Body::from(std::mem::replace(
                &mut self.buf,
                Vec::new(),
            )))
            .unwrap();

        let https = hyper_tls::HttpsConnector::new(1).unwrap();
        let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);

        let f = Box::new(
            client
                .request(req)
                .map_err(|e| format!("{:?}", e))
                .and_then(|res| {
                    if res.status() == hyper::StatusCode::OK {
                        return future::ok(());
                    }
                    future::err(format!("bad status: {:?}", res.status()))
                }),
        );

        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let output = match runtime.block_on(f) {
            Ok(m) => m,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("file does not exist: {:?}", e),
                ))
            }
        };

        self.size += self.buf.len() as u64;
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
            GFile::RemoteFile(f) => Ok(()),
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
    extern crate sstable;

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
    fn test_write_sstable() {
        let access = String::from("abcdef");
        let client = auth_client::AuthClient::new("127.0.0.1", 8888);
        client.global_init(access);
        {
            let mut f = GFile::create("/cns/colossus/data.sstable").unwrap();
            let mut t = sstable::SSTableBuilder::new(&mut f);
            t.write_ordered("abcdef", primitive::Primitive::from(0 as u64));
            t.finish().unwrap();
        }
    }

    //#[test]
    fn test_read_sstable() {
        let access = String::from("abcdef");
        let client = auth_client::AuthClient::new("127.0.0.1", 8888);
        client.global_init(access);
        {
            let mut f = GFile::open("/cns/colossus/data.sstable").unwrap();
            let mut t =
                sstable::SSTableReader::<primitive::Primitive<u64>>::new(Box::new(f)).unwrap();
            let output = t.collect::<Vec<_>>();
            assert_eq!(
                output,
                vec![(String::from("abcdef"), primitive::Primitive::from(0 as u64))]
            );
        }
    }
}
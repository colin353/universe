use auth_client::AuthServer;
use futures::future;
use futures::future::Future;
use futures::stream::Stream;
use std::fs::File;
use std::path::Path;

use std::io::Write;

const STORAGE_API: &'static str = "https://storage.googleapis.com/storage/v1/b";
const UPLOAD_API: &'static str = "https://storage.googleapis.com/upload/storage/v1/b";
const BUCKET: &'static str = "colossus";
const BUFFER_SIZE: usize = 1048576;

pub enum GFile {
    LocalFile(std::fs::File),
    RemoteFile(GoogleCloudFile),
}

pub struct GoogleCloudFile {
    bucket: String,
    object: String,
    token: String,
    resumable_url: Option<String>,
    buf: Vec<u8>,
    bytes_written: u64,
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
    pub fn open<P: AsRef<Path>>(token: &str, path: P) -> std::io::Result<GFile> {
        match GPath::from_path(path.as_ref()) {
            GPath::LocalPath(p) => {
                let f = std::fs::File::open(path)?;
                Ok(GFile::LocalFile(f))
            }
            GPath::RemotePath(bucket, object) => {
                let f = GoogleCloudFile::open(token, bucket, object)?;
                Ok(GFile::RemoteFile(f))
            }
        }
    }

    pub fn create<P: AsRef<Path>>(token: &str, path: P) -> std::io::Result<GFile> {
        match GPath::from_path(path.as_ref()) {
            GPath::LocalPath(p) => {
                let f = std::fs::File::create(path)?;
                Ok(GFile::LocalFile(f))
            }
            GPath::RemotePath(bucket, object) => {
                let f = GoogleCloudFile::create(token, bucket, object)?;
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

        Ok(Self {
            token: token.to_string(),
            bucket: bucket.to_string(),
            object: object.to_string(),
            resumable_url: None,
            buf: Vec::new(),
            bytes_written: 0,
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
            bytes_written: 0,
        })
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
        let end_range = self.bytes_written + self.buf.len() as u64;
        let content_range = if finished {
            format!(
                "bytes {}-{}/{}",
                self.bytes_written,
                end_range - 1,
                end_range
            )
        } else {
            format!("bytes {}-{}/*", self.bytes_written, end_range - 1)
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

        self.bytes_written += self.buf.len() as u64;
        self.buf.clear();

        Ok(())
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

impl Drop for GFile {
    fn drop(&mut self) {
        match self {
            GFile::RemoteFile(f) => f.flush(true).unwrap(),
            _ => return,
        };
    }
}

#[cfg(test)]
mod tests {
    #[macro_use]
    use super::*;

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
        let access = std::fs::read_to_string("/home/colin/.x20/auth_token").unwrap();
        let access = String::from("abcdef");
        let client = auth_client::AuthClient::new("127.0.0.1", 8888);
        let response = client.get_gcp_token(access);

        {
            let mut f =
                GFile::create(response.get_gcp_token(), "/cns/colossus/my_new_test.txt").unwrap();
            f.write(&[1, 2, 3, 4, 5, 6, 7]).unwrap();
        }
    }
}

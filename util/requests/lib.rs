use futures::future;
use futures::future::Future;
use futures::stream::Stream;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Response {
    status_code: u16,
    headers: hyper::HeaderMap,
    body: Vec<u8>,
}

impl Response {
    pub fn new() -> Self {
        Response {
            status_code: 0,
            headers: hyper::HeaderMap::new(),
            body: Vec::new(),
        }
    }
}

pub fn request(req: hyper::Request<hyper::Body>) -> std::io::Result<Response> {
    let https = hyper_tls::HttpsConnector::new(1).unwrap();
    let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);

    let mut response = Arc::new(Mutex::new(Response::new()));
    {
        let response = response.clone();
        let response2 = response.clone();

        let f = Box::new(
            client
                .request(req)
                .map_err(|e| ())
                .and_then(move |res| {
                    let mut r = response.lock().unwrap();
                    r.headers = res.headers().clone();
                    r.status_code = res.status().as_u16();

                    return future::ok(res.into_body().concat2().map_err(|_| ()));
                })
                .and_then(|res| res)
                .and_then(move |res| {
                    let mut r = response2.lock().unwrap();
                    r.body = res.into_bytes().to_vec();

                    future::ok(())
                }),
        );

        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        match runtime.block_on(f) {
            Ok(_) => (),
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "request failed",
                ))
            }
        }
    }

    Ok(Arc::try_unwrap(response).unwrap().into_inner().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    //#[test]
    fn test_make_request() {
        let req = hyper::Request::get("https://news.ycombinator.com/")
            .body(hyper::Body::from(String::new()))
            .unwrap();

        let response = request(req).unwrap();
        assert_eq!(response.status_code, 200);
    }
}

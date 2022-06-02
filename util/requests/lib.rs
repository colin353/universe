use hyper::body::Buf;

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub headers: hyper::HeaderMap,
    pub body: Vec<u8>,
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
    let https = hyper_tls::HttpsConnector::new();
    let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);

    {
        let f = async move {
            let res = client.request(req).await?;
            let mut r = Response::new();
            r.headers = res.headers().clone();
            r.status_code = res.status().as_u16();
            let mut body = hyper::body::aggregate(res).await?;
            r.body = body.to_bytes().to_vec();
            let result: Result<Response, hyper::Error> = Ok(r);
            result
        };

        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        match runtime.block_on(f) {
            Ok(x) => Ok(x),
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "request failed",
                ))
            }
        }
    }
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

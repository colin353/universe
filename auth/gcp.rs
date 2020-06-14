use futures::future;
use hyper::rt::{Future, Stream};

fn get_token(access_json: &str) -> Box<dyn Future<Item = String, Error = String> + Send> {
    let access = json::parse(access_json).unwrap();

    let body = hyper::Body::from("");

    let req = hyper::Request::post(access["token_uri"].as_str().unwrap())
        .header(
            hyper::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(hyper::Body::from(body))
        .unwrap();

    let https = hyper_tls::HttpsConnector::new(1).unwrap();
    let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);

    Box::new(
        client
            .request(req)
            .and_then(|res| res.into_body().concat2())
            .and_then(move |response| {
                let response = String::from_utf8(response.into_bytes().to_vec()).unwrap();
                future::ok(response)
            })
            .map_err(|f| format!("fail: {:?}", f)),
    )
}

#[cfg(test)]
mod tests {
    #[macro_use]
    use tokio;

    use super::*;
    //#[test]
    fn test_get_token() {
        let access = r#"
            {
                "token_uri": "https://oauth2.googleapis.com/token"
            }
        "#;

        tokio::run(
            get_token(access)
                .and_then(|resp| {
                    panic!("resp = {}", resp);
                    future::ok(())
                })
                .map_err(|_| ()),
        );

        panic!("done");
    }
}

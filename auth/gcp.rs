#[macro_use]
extern crate json;

use openssl::hash::MessageDigest;
use openssl::pkey;

use futures::future;
use hyper::rt::{Future, Stream};

const GOOGLE_RS256_HEAD: &str = r#"{"alg":"RS256","typ":"JWT"}"#;

fn get_timestamp() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs()
}

fn get_token(
    access_json: &str,
    scopes: &[&str],
) -> Box<dyn Future<Item = String, Error = String> + Send> {
    let access = json::parse(access_json).unwrap();

    // Create the claims JWT
    let mut claims = json::JsonValue::new_object();
    claims["iss"] = access["client_email"].clone();
    claims["aud"] = access["token_uri"].clone();
    claims["exp"] = (get_timestamp() + 3600 - 5).into();
    claims["iat"] = get_timestamp().into();
    claims["scope"] = scopes.join(" ").into();

    let mut encoded_claims = String::new();
    base64::encode_config_buf(GOOGLE_RS256_HEAD, base64::URL_SAFE, &mut encoded_claims);
    encoded_claims.push_str(".");
    base64::encode_config_buf(
        &json::stringify(claims),
        base64::URL_SAFE,
        &mut encoded_claims,
    );

    // Sign the encoded claims
    let private_key =
        pkey::PKey::private_key_from_pem(access["private_key"].as_str().unwrap().as_bytes())
            .unwrap();

    let mut signer = openssl::sign::Signer::new(MessageDigest::sha256(), &private_key).unwrap();
    signer.update(encoded_claims.as_bytes()).unwrap();
    let signature = signer.sign_to_vec().unwrap();
    encoded_claims.push_str(".");
    base64::encode_config_buf(&signature, base64::URL_SAFE, &mut encoded_claims);

    let body = hyper::Body::from(format!(
        "assertion={}\
         &grant_type={}",
        ws_utils::urlencode(&encoded_claims),
        ws_utils::urlencode("urn:ietf:params:oauth:grant-type:jwt-bearer"),
    ));

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
        let access = std::fs::read_to_string("/home/colin/security/bazel-access.json").unwrap();

        tokio::run(
            get_token(
                &access,
                &["https://www.googleapis.com/auth/devstorage.read_write"],
            )
            .and_then(|resp| {
                panic!("resp = {}", resp);
                future::ok(())
            })
            .map_err(|_| ()),
        );

        panic!("done");
    }
}

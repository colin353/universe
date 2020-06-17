#[macro_use]
extern crate json;

use openssl::hash::MessageDigest;
use openssl::pkey;

use futures::{future, Future};
use hyper::rt::Stream;

const GOOGLE_RS256_HEAD: &str = r#"{"alg":"RS256","typ":"JWT"}"#;

fn get_timestamp() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs()
}

pub fn get_token_sync(access_json: &str, scopes: &[&str]) -> (String, u64) {
    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    let output = runtime.block_on(get_token(&access_json, scopes));

    output.unwrap()
}

pub fn get_token(
    access_json: &str,
    scopes: &[&str],
) -> Box<dyn Future<Item = (String, u64), Error = String> + Send> {
    let access = json::parse(access_json).unwrap();

    let expiration_time = get_timestamp() + 3600 - 5;

    // Create the claims JWT
    let mut claims = json::JsonValue::new_object();
    claims["iss"] = access["client_email"].clone();
    claims["aud"] = access["token_uri"].clone();
    claims["exp"] = expiration_time.into();
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
                let parsed = json::parse(&response).unwrap();
                future::ok((
                    parsed["access_token"].as_str().unwrap().to_string(),
                    expiration_time,
                ))
            })
            .map_err(|f| format!("fail: {:?}", f)),
    )
}

#[cfg(test)]
mod tests {
    #[macro_use]
    use tokio;

    use super::*;
    #[test]
    fn test_get_token() {
        let access = std::fs::read_to_string("/home/colin/security/bazel-access.json").unwrap();

        let mut runtime = tokio::runtime::Runtime::new().unwrap();
        let output = runtime.block_on(get_token(
            &access,
            &["https://www.googleapis.com/auth/devstorage.read_write"],
        ));

        output.unwrap();
    }
}

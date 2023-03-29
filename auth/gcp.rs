use hyper::body::Buf;

use openssl::hash::MessageDigest;
use openssl::pkey;

const GOOGLE_RS256_HEAD: &str = r#"{"alg":"RS256","typ":"JWT"}"#;

fn get_timestamp() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs()
}

pub async fn get_token(access_json: &str, scopes: &[&str]) -> Result<(String, u64), hyper::Error> {
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

    let https = hyper_tls::HttpsConnector::new();
    let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);

    let resp = client.request(req).await?;
    let mut body = hyper::body::aggregate(resp).await?;
    let bytes = body.to_bytes();
    let response = std::str::from_utf8(&bytes).unwrap();
    println!("response = {}", response);
    let parsed = json::parse(&response).unwrap();
    Ok((
        parsed["access_token"].as_str().unwrap().to_string(),
        expiration_time,
    ))
}

#[cfg(test)]
mod tests {
    use tokio;

    use super::*;
    //#[test]
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

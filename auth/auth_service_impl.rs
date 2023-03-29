use hyper::body::Buf;
use hyper::header::HeaderValue;
use hyper::StatusCode;
use hyper_tls::HttpsConnector;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ws::{Body, Request, Response, ResponseFuture, Server};

const MACHINE_USERNAME: &str = "service-account";

pub struct LoginRecord {
    username: String,
    state: String,
    valid: bool,
    return_url: String,
}

impl LoginRecord {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            state: String::new(),
            valid: false,
            return_url: String::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

#[derive(Clone)]
pub struct AuthServiceHandler {
    hostname: String,
    oauth_client_id: String,
    tokens: Arc<RwLock<HashMap<String, LoginRecord>>>,
    secret_key: String,
    default_access_json: String,
}

impl AuthServiceHandler {
    pub fn new(
        hostname: String,
        oauth_client_id: String,
        tokens: Arc<RwLock<HashMap<String, LoginRecord>>>,
        secret_key: String,
        default_access_json: String,
    ) -> Self {
        Self {
            hostname,
            tokens,
            oauth_client_id,
            secret_key,
            default_access_json,
        }
    }
}

fn random_string() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{}{}{}{}",
        rng.gen::<u64>(),
        rng.gen::<u64>(),
        rng.gen::<u64>(),
        rng.gen::<u64>()
    )
}

impl AuthServiceHandler {
    async fn login(
        &self,
        req: auth_bus::LoginRequest,
    ) -> Result<auth_bus::LoginChallenge, bus::BusRpcError> {
        let mut challenge = auth_bus::LoginChallenge::new();
        let token = random_string();
        let url = format!("{}begin?token={}", self.hostname, token);
        challenge.url = url;
        challenge.token = token.clone();

        let mut record = LoginRecord::new();
        record.return_url = req.return_url;
        self.tokens.write().unwrap().insert(token, record);

        Ok(challenge)
    }

    async fn get_gcp_token(
        &self,
        req: auth_bus::GCPTokenRequest,
    ) -> Result<auth_bus::GCPTokenResponse, bus::BusRpcError> {
        let mut response = auth_bus::GCPTokenResponse::new();

        let mut authenticated = false;
        if req.token == self.secret_key {
            authenticated = true;
        } else if let Some(t) = self.tokens.read().unwrap().get(&req.token) {
            if t.is_valid() {
                authenticated = true;
            }
        }

        if !authenticated {
            return Ok(response);
        }

        let (token, expiry) = match gcp::get_token(
            &self.default_access_json,
            &["https://www.googleapis.com/auth/devstorage.read_write"],
        )
        .await
        {
            Ok(z) => z,
            Err(e) => {
                eprintln!("something went wrong while conducting oauth! {:?}", e);
                return Ok(response);
            }
        };

        response.success = true;
        response.gcp_token = token;
        response.expiry = expiry;

        Ok(response)
    }

    async fn authenticate(
        &self,
        req: auth_bus::AuthenticateRequest,
    ) -> Result<auth_bus::AuthenticateResponse, bus::BusRpcError> {
        let mut response = auth_bus::AuthenticateResponse::new();
        if req.token == self.secret_key {
            response.success = true;
            response.username = MACHINE_USERNAME.to_owned();
        } else if let Some(t) = self.tokens.read().unwrap().get(&req.token) {
            if t.is_valid() {
                response.success = true;
                response.username = t.username.clone();
            }
        }
        Ok(response)
    }
}

impl auth_bus::AuthenticationAsyncServiceHandler for AuthServiceHandler {
    fn login(
        &self,
        req: auth_bus::LoginRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<auth_bus::LoginChallenge, bus::BusRpcError>>
                + Send,
        >,
    > {
        let _self = self.clone();
        Box::pin(async move { _self.login(req).await })
    }

    fn authenticate(
        &self,
        req: auth_bus::AuthenticateRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<auth_bus::AuthenticateResponse, bus::BusRpcError>,
                > + Send,
        >,
    > {
        let _self = self.clone();
        Box::pin(async move { _self.authenticate(req).await })
    }

    fn get_gcp_token(
        &self,
        req: auth_bus::GCPTokenRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<auth_bus::GCPTokenResponse, bus::BusRpcError>>
                + Send,
        >,
    > {
        let _self = self.clone();
        Box::pin(async move { _self.get_gcp_token(req).await })
    }
}

#[derive(Clone)]
pub struct AuthWebServer {
    tokens: Arc<RwLock<HashMap<String, LoginRecord>>>,
    hostname: String,
    cookie_domain: String,
    client_id: String,
    client_secret: String,
    email_whitelist: Arc<HashMap<String, String>>,
}

impl AuthWebServer {
    pub fn new(
        tokens: Arc<RwLock<HashMap<String, LoginRecord>>>,
        hostname: String,
        cookie_domain: String,
        client_id: String,
        client_secret: String,
        email_whitelist: Arc<HashMap<String, String>>,
    ) -> Self {
        Self {
            tokens: tokens,
            hostname: hostname,
            cookie_domain: cookie_domain,
            client_id: client_id,
            client_secret: client_secret,
            email_whitelist: email_whitelist,
        }
    }

    fn respond_error(&self) -> ResponseFuture {
        Box::pin(std::future::ready(Response::new(Body::from("invalid"))))
    }

    fn begin_authentication(&self, _path: String, req: Request, _key: &str) -> ResponseFuture {
        let query = match req.uri().query() {
            Some(q) => q,
            None => return self.respond_error(),
        };

        let params = ws_utils::parse_params(query);
        let token = match params.get("token") {
            Some(x) => x,
            None => return self.respond_error(),
        };

        // Construct the challenge URL
        let redirect_uri = ws_utils::urlencode(&format!("{}finish", self.hostname));
        let state = random_string();
        let url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
             client_id={client_id}&\
             response_type=code&\
             scope=openid%20email&\
             redirect_uri={redirect_uri}&\
             state={state}&\
             nonce={nonce}",
            client_id = self.client_id,
            redirect_uri = redirect_uri,
            state = state,
            nonce = random_string(),
        );

        let mut response = Response::new(Body::from("redirecting..."));
        *response.status_mut() = StatusCode::TEMPORARY_REDIRECT;
        {
            let headers = response.headers_mut();
            headers.insert("Location", HeaderValue::from_str(&url).unwrap());
        }

        self.set_cookie_for_domain(token, &self.cookie_domain, &mut response);
        Box::pin(std::future::ready(response))
    }

    fn finish_authentication(&self, path: String, req: Request, key: String) -> ResponseFuture {
        let _self = self.clone();
        Box::pin(async move { _self.async_finish(path, req, key).await.unwrap() })
    }

    async fn async_finish(
        &self,
        _path: String,
        req: Request,
        key: String,
    ) -> Result<Response, hyper::Error> {
        let query = match req.uri().query() {
            Some(q) => q,
            None => return Ok(Response::new(Body::from(""))),
        };

        let params = ws_utils::parse_params(query);
        let redirect_uri = ws_utils::urlencode(&format!("{}finish", self.hostname));
        let body = format!(
            "code={code}\
                 &client_id={client_id}\
                 &client_secret={client_secret}\
                 &redirect_uri={redirect_uri}\
                 &grant_type=authorization_code",
            code = params.get("code").unwrap(),
            client_id = self.client_id,
            client_secret = self.client_secret,
            redirect_uri = redirect_uri,
        );

        let req = hyper::Request::builder();
        let req = req
            .method("POST")
            .uri("https://www.googleapis.com/oauth2/v4/token/");
        let mut req = req.body(hyper::Body::from(body)).unwrap();

        {
            let headers = req.headers_mut();
            headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
        }

        let https = HttpsConnector::new();
        let client = hyper::client::Client::builder().build::<_, hyper::Body>(https);
        let tokens = self.tokens.clone();
        let email_whitelist = self.email_whitelist.clone();

        let resp = client.request(req).await?;
        let mut body = hyper::body::aggregate(resp).await?;
        let bytes = body.to_bytes();

        let response = std::str::from_utf8(&bytes).unwrap();
        let parsed = json::parse(&response).unwrap();
        let token = &parsed["access_token"];

        let req = hyper::Request::builder();
        let req = req
            .method("GET")
            .uri("https://openidconnect.googleapis.com/v1/userinfo");
        let mut req = req.body(hyper::Body::from("")).unwrap();

        {
            let headers = req.headers_mut();
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
            );
        }

        let resp = client.request(req).await?;
        let mut body = hyper::body::aggregate(resp).await?;
        let bytes = body.to_bytes();
        let response = std::str::from_utf8(&bytes).unwrap();
        let parsed = json::parse(&response).unwrap();
        let email = &parsed["email"];

        let email_str = match email.as_str() {
            Some(e) => e,
            None => {
                return Ok(Response::new(Body::from("invalid")));
            }
        };

        let username = match email_whitelist.get(email_str) {
            Some(x) => x,
            None => return Ok(Response::new(Body::from("invalid"))),
        };

        let mut tokens_write = tokens.write().unwrap();
        let login_record = match tokens_write.get_mut(&key) {
            Some(x) => x,
            None => {
                return Ok(Response::new(Body::from("invalid")));
            }
        };

        if !login_record.username.is_empty() && &login_record.username != username {
            return Ok(Response::new(Body::from("invalid")));
        }
        login_record.valid = true;
        login_record.username = username.to_owned();

        let mut response = Response::new(Body::from(format!("{}", email)));
        *response.status_mut() = StatusCode::TEMPORARY_REDIRECT;
        {
            let headers = response.headers_mut();
            headers.insert(
                "Location",
                HeaderValue::from_str(&login_record.return_url).unwrap(),
            );
        }

        Ok(response)
    }
}

impl Server for AuthWebServer {
    fn respond_future(&self, path: String, req: Request, key: &str) -> ResponseFuture {
        if path.starts_with("/finish") {
            return self.finish_authentication(path, req, key.to_owned());
        }

        return self.begin_authentication(path, req, key);
    }
}

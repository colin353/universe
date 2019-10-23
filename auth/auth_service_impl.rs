extern crate hyper;
extern crate rand;
extern crate ws;
extern crate ws_utils;

use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ws::{Body, Request, Response, Server};

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
}
impl AuthServiceHandler {
    pub fn new(
        hostname: String,
        oauth_client_id: String,
        tokens: Arc<RwLock<HashMap<String, LoginRecord>>>,
    ) -> Self {
        Self {
            hostname: hostname,
            tokens: tokens,
            oauth_client_id: oauth_client_id,
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

impl auth_grpc_rust::AuthenticationService for AuthServiceHandler {
    fn login(
        &self,
        _m: grpc::RequestOptions,
        mut req: auth_grpc_rust::LoginRequest,
    ) -> grpc::SingleResponse<auth_grpc_rust::LoginChallenge> {
        let redirect_uri = ws_utils::urlencode(&self.hostname);
        // Construct the challenge URL
        let state = random_string();
        let url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
             client_id={client_id}&\
             response_type=code&\
             scope=openid%20email&\
             redirect_uri={redirect_uri}&\
             state={state}&\
             nonce={nonce}",
            client_id = self.oauth_client_id,
            redirect_uri = redirect_uri,
            state = state,
            nonce = random_string(),
        );
        let mut challenge = auth_grpc_rust::LoginChallenge::new();
        let token = random_string();
        challenge.set_url(url);
        challenge.set_token(token.clone());

        let mut record = LoginRecord::new();
        record.state = state;
        record.return_url = req.take_return_url();
        self.tokens.write().unwrap().insert(token, record);

        grpc::SingleResponse::completed(challenge)
    }

    fn authenticate(
        &self,
        _m: grpc::RequestOptions,
        req: auth_grpc_rust::AuthenticateRequest,
    ) -> grpc::SingleResponse<auth_grpc_rust::AuthenticateResponse> {
        let mut response = auth_grpc_rust::AuthenticateResponse::new();
        if let Some(t) = self.tokens.read().unwrap().get(req.get_token()) {
            if t.is_valid() {
                response.set_success(true);
                response.set_username(t.username.clone());
            }
        }
        grpc::SingleResponse::completed(response)
    }
}

#[derive(Clone)]
pub struct AuthWebServer {
    tokens: Arc<RwLock<HashMap<String, LoginRecord>>>,
}

impl AuthWebServer {
    pub fn new(tokens: Arc<RwLock<HashMap<String, LoginRecord>>>) -> Self {
        Self { tokens: tokens }
    }
}

impl Server for AuthWebServer {
    fn respond(&self, path: String, req: Request, _: &str) -> Response {
        let query = match req.uri().query() {
            Some(q) => q,
            None => return Response::new(Body::from("")),
        };

        let params = ws_utils::parse_params(query);
        Response::new(Body::from(format!("{:?}", params)))
    }
}

use hyper::http::Uri;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

struct LoginRecord {
    username: String,
    valid: bool,
}
impl LoginRecord {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            state: String::new(),
            valid: false,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

#[derive(Clone)]
pub struct AuthServiceHandler {
    hostname: String,
    tokens: Arc<RwLock<HashMap<String, LoginRecord>>>,
}
impl AuthServiceHandler {
    pub fn new(hostname: String, tokens: Arc<RwLock<HashMap>>) -> Self {
        Self {
            hostname: hostname,
            tokens: tokens,
        }
    }
}

fn random_string() -> String {
    format!("{}{}{}{}", rng.gen::<u64>(), rng.gen::<u64>(), rng.gen::<u64>(), rng.gen::<u64>())
}

impl auth_grpc_rust::AuthenticationService for AuthServiceHandler {
    fn login(
        &self,
        _m: grpc::RequestOptions,
        req: auth_grpc_rust::LoginRequest,
    ) -> grpc::SingleResponse<auth_grpc_rust::LoginChallenge> {
        let redirect_uri = "http%3A%2F%2Fauth.colinmerkel.xyz";
        // Construct the challenge URL
        let state = random_string();
        let url = format!("https://accounts.google.com/o/oauth2/v2/auth?\
            client_id={client_id}&\
            response_type=code&\
            scope=openid%20email&\
            redirect_uri={redirect_uri}&\
            state={state}&\
            nonce={nonce}",
            client_id=self.oauth_client_id,
            redirect_uri=redirect_uri,
            state=state,
            nonce=random_string(),
        )
        let mut challenge = auth_grpc_rust::LoginChallenge::new();
        let token = random_string();
        challenge.set_url(url);
        challenge.set_token(token.clone());

        let mut record = LoginRecord::new();
        record.state = state;
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

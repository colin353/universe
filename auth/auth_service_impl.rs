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
    pub fn new(hostname: String) -> Self {
        Self {
            hostname: hostname,
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl auth_grpc_rust::AuthenticationService for AuthServiceHandler {
    fn login(
        &self,
        _m: grpc::RequestOptions,
        req: auth_grpc_rust::LoginRequest,
    ) -> grpc::SingleResponse<auth_grpc_rust::LoginChallenge> {
        grpc::SingleResponse::completed(auth_grpc_rust::LoginChallenge::new())
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

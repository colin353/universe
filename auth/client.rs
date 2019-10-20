extern crate auth_grpc_rust;
extern crate grpc;
pub use auth_grpc_rust::*;
use std::sync::Arc;

pub trait AuthServer: Send + Sync + Clone + 'static {
    fn authenticate(&self, token: String) -> AuthenticateResponse;
    fn login(&self) -> LoginChallenge;
}

#[derive(Clone)]
pub struct AuthClient {
    client: Arc<AuthenticationServiceClient>,
}

impl AuthClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Arc::new(
                AuthenticationServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            ),
        }
    }

    fn opts(&self) -> grpc::RequestOptions {
        grpc::RequestOptions::new()
    }
}

impl AuthServer for AuthClient {
    fn authenticate(&self, token: String) -> AuthenticateResponse {
        let mut req = AuthenticateRequest::new();
        req.set_token(token);
        self.client
            .authenticate(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    fn login(&self) -> LoginChallenge {
        self.client
            .login(self.opts(), LoginRequest::new())
            .wait()
            .expect("rpc")
            .1
    }
}

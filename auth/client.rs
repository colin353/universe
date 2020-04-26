extern crate auth_grpc_rust;
extern crate grpc;

pub use auth_grpc_rust::*;
use grpc::{ClientStub, ClientStubExt};
use std::sync::Arc;

pub trait AuthServer: Send + Sync + Clone + 'static {
    fn authenticate(&self, token: String) -> AuthenticateResponse;
    fn login(&self) -> LoginChallenge;
    fn login_then_redirect(&self, return_url: String) -> LoginChallenge;
}

#[derive(Clone)]
pub struct AuthClient {
    client: Option<Arc<AuthenticationServiceClient>>,
}

impl AuthClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Some(Arc::new(
                AuthenticationServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            )),
        }
    }

    pub fn new_tls(hostname: &str, port: u16) -> Self {
        let grpc_client = grpc_tls::make_tls_client(hostname, port);
        Self {
            client: Some(Arc::new(AuthenticationServiceClient::with_client(
                Arc::new(grpc_client),
            ))),
        }
    }

    pub fn new_fake() -> Self {
        Self { client: None }
    }

    fn opts(&self) -> grpc::RequestOptions {
        grpc::RequestOptions::new()
    }
}

impl AuthServer for AuthClient {
    fn authenticate(&self, token: String) -> AuthenticateResponse {
        if self.client.is_none() {
            let mut response = AuthenticateResponse::new();
            response.set_username(String::from("fake-user"));
            response.set_success(true);
            return response;
        }

        let mut req = AuthenticateRequest::new();
        req.set_token(token);
        let result = self
            .client
            .as_ref()
            .unwrap()
            .authenticate(self.opts(), req)
            .wait()
            .expect("rpc")
            .1;
        result
    }

    fn login(&self) -> LoginChallenge {
        self.client
            .as_ref()
            .unwrap()
            .login(self.opts(), LoginRequest::new())
            .wait()
            .expect("rpc")
            .1
    }

    fn login_then_redirect(&self, return_url: String) -> LoginChallenge {
        let mut req = LoginRequest::new();
        req.set_return_url(return_url);
        self.client
            .as_ref()
            .unwrap()
            .login(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }
}

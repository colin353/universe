extern crate auth_grpc_rust;
extern crate grpc;

#[macro_use]
extern crate lazy_static;

pub use auth_grpc_rust::*;
use cache::Cache;
use grpc::{ClientStub, ClientStubExt};
use std::sync::{Arc, RwLock};

fn wait<T: Send + Sync>(resp: grpc::SingleResponse<T>) -> Result<T, grpc::Error> {
    futures::executor::block_on(resp.join_metadata_result()).map(|r| r.1)
}

lazy_static! {
    static ref GLOBAL_CLIENT: RwLock<Option<AuthClient>> = RwLock::new(None);
}

pub fn get_timestamp() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs()
}

pub trait AuthServer: Send + Sync + Clone + 'static {
    fn authenticate(&self, token: String) -> AuthenticateResponse;
    fn login(&self) -> LoginChallenge;
    fn login_then_redirect(&self, return_url: String) -> LoginChallenge;
    fn get_gcp_token(&self, token: String) -> GCPTokenResponse;
}

#[derive(Clone)]
pub struct AuthClient {
    pub token: String,
    client: Option<Arc<AuthenticationServiceClient>>,
    auth_cache: Arc<cache::Cache<String, AuthenticateResponse>>,
    gcp_cache: Arc<cache::Cache<String, GCPTokenResponse>>,
}

pub fn get_global_client() -> Option<AuthClient> {
    match &*(GLOBAL_CLIENT.read().unwrap()) {
        Some(c) => Some(c.clone()),
        None => None,
    }
}

impl AuthClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Some(Arc::new(
                AuthenticationServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            )),
            auth_cache: Arc::new(Cache::new(4096)),
            gcp_cache: Arc::new(Cache::new(4096)),
            token: String::new(),
        }
    }

    pub fn global_init(&self, token: String) {
        let mut c = self.clone();
        c.token = token;
        let mut gc = GLOBAL_CLIENT.write().unwrap();
        *gc = Some(c);
    }

    pub fn upgrade_auth_to_gcloud_token(&self) {}

    pub fn new_tls(hostname: &str, port: u16) -> Self {
        let grpc_client = grpc_tls::make_tls_client(hostname, port);
        Self {
            client: Some(Arc::new(AuthenticationServiceClient::with_client(
                Arc::new(grpc_client),
            ))),
            auth_cache: Arc::new(Cache::new(4096)),
            gcp_cache: Arc::new(Cache::new(4096)),
            token: String::new(),
        }
    }

    pub fn new_fake() -> Self {
        Self {
            client: None,
            auth_cache: Arc::new(Cache::new(16)),
            gcp_cache: Arc::new(Cache::new(16)),
            token: String::new(),
        }
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

        if let Some(r) = self.auth_cache.get(&token) {
            return r;
        }

        let mut req = AuthenticateRequest::new();
        req.set_token(token.clone());
        let result = wait(self.client.as_ref().unwrap().authenticate(self.opts(), req)).unwrap();

        if result.get_success() {
            self.auth_cache.insert(token, result.clone());
        }

        result
    }

    fn login(&self) -> LoginChallenge {
        wait(
            self.client
                .as_ref()
                .unwrap()
                .login(self.opts(), LoginRequest::new()),
        )
        .unwrap()
    }

    fn get_gcp_token(&self, token: String) -> GCPTokenResponse {
        if let Some(r) = self.gcp_cache.get(&token) {
            if get_timestamp() + 1800 < r.get_expiry() {
                return r;
            }
        }

        let mut req = GCPTokenRequest::new();
        req.set_token(token);

        wait(
            self.client
                .as_ref()
                .unwrap()
                .get_gcp_token(self.opts(), req),
        )
        .unwrap()
    }

    fn login_then_redirect(&self, return_url: String) -> LoginChallenge {
        let mut req = LoginRequest::new();
        req.set_return_url(return_url);
        wait(self.client.as_ref().unwrap().login(self.opts(), req)).unwrap()
    }
}

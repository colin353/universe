#[macro_use]
extern crate lazy_static;

pub use auth_bus::*;
use cache::Cache;
use std::sync::{Arc, RwLock};

mod async_client;
pub use async_client::AuthAsyncClient;

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
    client: Option<Arc<AuthenticationClient>>,
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
        let connector =
            std::sync::Arc::new(bus_rpc::HyperSyncClient::new(hostname.to_string(), port));
        Self {
            client: Some(Arc::new(AuthenticationClient::new(connector))),
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

    pub fn new_tls(hostname: &str, port: u16) -> Self {
        let connector = std::sync::Arc::new(bus_rpc::HyperSyncClient::new_tls(
            hostname.to_string(),
            port,
        ));
        Self {
            client: Some(Arc::new(AuthenticationClient::new(connector))),
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
}

impl AuthServer for AuthClient {
    fn authenticate(&self, token: String) -> AuthenticateResponse {
        if self.client.is_none() {
            let mut response = AuthenticateResponse::new();
            response.username = String::from("fake-user");
            response.success = true;
            return response;
        }

        if let Some(r) = self.auth_cache.get(&token) {
            return r;
        }

        let mut req = AuthenticateRequest::new();
        req.token = token.clone();
        let result = self.client.as_ref().unwrap().authenticate(req).unwrap();

        if result.success {
            self.auth_cache.insert(token, result.clone());
        }

        result
    }

    fn login(&self) -> LoginChallenge {
        self.client
            .as_ref()
            .unwrap()
            .login(LoginRequest::new())
            .unwrap()
    }

    fn get_gcp_token(&self, token: String) -> GCPTokenResponse {
        if let Some(r) = self.gcp_cache.get(&token) {
            if get_timestamp() + 1800 < r.expiry {
                return r;
            }
        }

        let mut req = GCPTokenRequest::new();
        req.token = token;

        self.client.as_ref().unwrap().get_gcp_token(req).unwrap()
    }

    fn login_then_redirect(&self, return_url: String) -> LoginChallenge {
        let mut req = LoginRequest::new();
        req.return_url = return_url;
        self.client.as_ref().unwrap().login(req).unwrap()
    }
}

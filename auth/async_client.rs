pub use auth_bus::*;
use cache::Cache;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthAsyncClient {
    pub token: String,
    client: Option<Arc<AuthenticationAsyncClient>>,
    auth_cache: Arc<cache::Cache<String, AuthenticateResponse>>,
    gcp_cache: Arc<cache::Cache<String, GCPTokenResponse>>,
}

impl AuthAsyncClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        let connector = std::sync::Arc::new(bus_rpc::HyperClient::new(hostname.to_string(), port));
        Self {
            client: Some(Arc::new(AuthenticationAsyncClient::new(connector))),
            auth_cache: Arc::new(Cache::new(4096)),
            gcp_cache: Arc::new(Cache::new(4096)),
            token: String::new(),
        }
    }

    pub fn new_tls(hostname: &str, port: u16) -> Self {
        let connector =
            std::sync::Arc::new(bus_rpc::HyperClient::new_tls(hostname.to_string(), port));
        Self {
            client: Some(Arc::new(AuthenticationAsyncClient::new(connector))),
            auth_cache: Arc::new(Cache::new(4096)),
            gcp_cache: Arc::new(Cache::new(4096)),
            token: String::new(),
        }
    }

    pub fn new_metal(service_name: &str) -> Self {
        let connector = std::sync::Arc::new(bus_rpc::MetalAsyncClient::new(service_name));
        Self {
            client: Some(Arc::new(AuthenticationAsyncClient::new(connector))),
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

    pub async fn authenticate(
        &self,
        token: String,
    ) -> Result<AuthenticateResponse, bus::BusRpcError> {
        if self.client.is_none() {
            let mut response = AuthenticateResponse::new();
            response.username = String::from("fake-user");
            response.success = true;
            return Ok(response);
        }

        if let Some(r) = self.auth_cache.get(&token) {
            return Ok(r);
        }

        let mut req = AuthenticateRequest::new();
        req.token = token.clone();
        match self.client.as_ref().unwrap().authenticate(req).await {
            Ok(r) => {
                if r.success {
                    self.auth_cache.insert(token, r.clone());
                }
                Ok(r)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn login(&self) -> LoginChallenge {
        self.client
            .as_ref()
            .unwrap()
            .login(LoginRequest::new())
            .await
            .unwrap()
    }

    pub async fn get_gcp_token(&self, token: String) -> GCPTokenResponse {
        if let Some(r) = self.gcp_cache.get(&token) {
            if crate::get_timestamp() + 1800 < r.expiry {
                return r;
            }
        }

        let mut req = GCPTokenRequest::new();
        req.token = token;

        self.client
            .as_ref()
            .unwrap()
            .get_gcp_token(req)
            .await
            .unwrap()
    }

    pub async fn login_then_redirect(&self, return_url: String) -> LoginChallenge {
        let mut req = LoginRequest::new();
        req.return_url = return_url;
        self.client.as_ref().unwrap().login(req).await.unwrap()
    }
}

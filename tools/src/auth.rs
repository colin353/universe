use std::future::Future;
use std::pin::Pin;

pub struct User {
    pub username: String,
}

pub trait AuthPlugin: Send + Sync {
    fn validate(&self, token: &str) -> Pin<Box<dyn Future<Output = Result<User, String>> + Send>>;
    fn discover(&self) -> Pin<Box<dyn Future<Output = service::DiscoverAuthResponse> + Send>>;
}

#[derive(Clone)]
pub struct FakeAuthPlugin {}

impl FakeAuthPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

impl AuthPlugin for FakeAuthPlugin {
    fn validate(&self, token: &str) -> Pin<Box<dyn Future<Output = Result<User, String>> + Send>> {
        return Box::pin(std::future::ready(Ok(User {
            username: token.to_string(),
        })));
    }

    fn discover(&self) -> Pin<Box<dyn Future<Output = service::DiscoverAuthResponse> + Send>> {
        return Box::pin(std::future::ready(service::DiscoverAuthResponse {
            auth_kind: service::AuthKind::None,
            ..Default::default()
        }));
    }
}

#[derive(Clone)]
pub struct AuthServicePlugin {
    client: auth_client::AuthAsyncClient,
    host: String,
    port: u16,
}

impl AuthServicePlugin {
    pub fn new(client: auth_client::AuthAsyncClient, host: String, port: u16) -> Self {
        Self { client, host, port }
    }
}

impl AuthPlugin for AuthServicePlugin {
    fn validate(&self, token: &str) -> Pin<Box<dyn Future<Output = Result<User, String>> + Send>> {
        let client = self.client.clone();
        let token = token.to_string();
        Box::pin(async move {
            let resp = client
                .authenticate(token)
                .await
                .map_err(|e| format!("failed to contact auth service: {e:?}"))?;
            if resp.success {
                return Ok(User {
                    username: resp.username,
                });
            }

            Err(String::from("authentication failed"))
        })
    }

    fn discover(&self) -> Pin<Box<dyn Future<Output = service::DiscoverAuthResponse> + Send>> {
        Box::pin(std::future::ready(service::DiscoverAuthResponse {
            auth_kind: service::AuthKind::AuthService,
            auth_service_host: self.host.clone(),
            auth_service_port: self.port,
        }))
    }
}

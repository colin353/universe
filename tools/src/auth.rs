use auth_client::AuthServer;

pub struct User {
    pub username: String,
}

pub trait AuthPlugin: Send + Sync {
    fn validate(&self, token: &str) -> Result<User, String>;
    fn discover(&self) -> service::DiscoverAuthResponse;
}

pub struct FakeAuthPlugin {}

impl FakeAuthPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

impl AuthPlugin for FakeAuthPlugin {
    fn validate(&self, token: &str) -> Result<User, String> {
        return Ok(User {
            username: token.to_string(),
        });
    }

    fn discover(&self) -> service::DiscoverAuthResponse {
        service::DiscoverAuthResponse {
            auth_kind: service::AuthKind::None,
            ..Default::default()
        }
    }
}

pub struct AuthServicePlugin {
    client: auth_client::AuthClient,
    host: String,
}

impl AuthServicePlugin {
    pub fn new(client: auth_client::AuthClient, host: String) -> Self {
        Self { client, host: host }
    }
}

impl AuthPlugin for AuthServicePlugin {
    fn validate(&self, token: &str) -> Result<User, String> {
        let resp = self.client.authenticate(token.to_string());
        if resp.get_success() {
            return Ok(User {
                username: resp.get_username().to_string(),
            });
        }

        Err(String::from("authentication failed"))
    }

    fn discover(&self) -> service::DiscoverAuthResponse {
        service::DiscoverAuthResponse {
            auth_kind: service::AuthKind::AuthService,
            auth_service_host: self.host.clone(),
        }
    }
}

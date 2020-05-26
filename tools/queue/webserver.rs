use largetable_client::LargeTableClient;

use auth_client::AuthServer;
use ws::{Body, Request, Response, Server};

#[derive(Clone)]
pub struct QueueWebServer<C: LargeTableClient + Send + Sync + Clone + 'static> {
    database: C,
    auth: auth_client::AuthClient,
    base_url: String,
}

impl<C: LargeTableClient + Clone + Send + Sync + 'static> QueueWebServer<C> {
    pub fn new(database: C, auth: auth_client::AuthClient, base_url: String) -> Self {
        Self {
            database,
            auth,
            base_url,
        }
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        Response::new(Body::from(String::from("hello world")))
    }
}

impl<C: LargeTableClient + Send + Sync + Clone + 'static> Server for QueueWebServer<C> {
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        let result = self.auth.authenticate(token.to_owned());
        if !result.get_success() {
            let challenge = self
                .auth
                .login_then_redirect(format!("{}{}", self.base_url, path));
            let mut response = Response::new(Body::from("redirect to login"));
            self.redirect(challenge.get_url(), &mut response);
            return response;
        }

        return self.index(path, req);
    }
}

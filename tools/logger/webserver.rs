#[macro_use]
extern crate tmpl;

use auth_client::AuthServer;
use ws::{Body, Request, Response, Server};

#[derive(Clone)]
pub struct LoggerWebServer {
    handler: server_lib::LoggerServiceHandler,
    auth: auth_client::AuthClient,
    base_url: String,
}

impl LoggerWebServer {
    pub fn new(
        handler: server_lib::LoggerServiceHandler,
        auth: auth_client::AuthClient,
        base_url: String,
    ) -> Self {
        Self {
            auth,
            handler,
            base_url,
        }
    }

    fn index(&self) -> Response {
        Response::new(Body::from(String::from("hello world")))
    }
}

impl Server for LoggerWebServer {
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

        return self.index();
    }
}

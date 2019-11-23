#[macro_use]
extern crate flags;
extern crate auth_client;
extern crate ws;
use ws::{Body, Request, Response, Server};

#[derive(Clone)]
pub struct HomepageServer<A> {
    static_dir: String,
    auth: A,
    base_url: String,
}
impl<A> HomepageServer<A>
where
    A: auth_client::AuthServer,
{
    fn new(static_dir: String, base_url: String, auth: A) -> Self {
        Self {
            static_dir: static_dir,
            auth: auth,
            base_url: base_url,
        }
    }

    fn login(&self, token: &str) -> Response {
        let result = self.auth.authenticate(token.to_owned());
        if result.get_success() {
            return Response::new(Body::from("you are logged"));
        }

        let result = self
            .auth
            .login_then_redirect(format!("{}/login-test", self.base_url));
        let mut response = Response::new(Body::from("redirecting..."));
        self.redirect(result.get_url(), &mut response);
        self.set_cookie(result.get_token(), &mut response);
        response
    }

    fn login_test(&self, token: &str) -> Response {
        let result = self.auth.authenticate(token.to_owned());
        if result.get_success() {
            return Response::new(Body::from("you are logged"));
        }
        Response::new(Body::from("you are NOT logged"))
    }
}

impl<A> Server for HomepageServer<A>
where
    A: auth_client::AuthServer,
{
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        if path.starts_with("/static/") {
            return self.serve_static_files(path, "/static/", &self.static_dir);
        }

        if path == "/login-test" {
            return self.login_test(token);
        }

        if path == "/login" {
            return self.login(token);
        }

        Response::new(Body::from("hello world!"))
    }
}

fn main() {
    let port = define_flag!("port", 8080, "the port to bind to");
    let static_files = define_flag!(
        "static_files",
        String::from("/static/"),
        "the directory containing static files"
    );
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("127.0.0.1"),
        "the hostname of the authentication service"
    );
    let base_url = define_flag!(
        "base_url",
        String::from("http://localhost:8080"),
        "the base url to access this service"
    );
    let auth_port = define_flag!("auth_port", 8888, "the port of the authentication service");
    parse_flags!(port, static_files, auth_hostname, auth_port, base_url);

    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());
    HomepageServer::new(static_files.value(), base_url.value(), auth).serve(port.value());
}

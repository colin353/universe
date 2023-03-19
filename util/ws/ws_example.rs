#[macro_use]
extern crate tmpl;
extern crate ws;
use ws::{Body, Request, Response, Server};

static MSG: &str = "Start svr: {}";
static TEMPLATE: &str = include_str!("template.html");

#[derive(Clone)]
struct ExampleServer {
    secret_code: String,
}

impl ExampleServer {
    fn new(secret_code: String) -> Self {
        ExampleServer { secret_code }
    }

    fn index(&self, path: String, req: Request) -> Response {
        let name = match req.uri().query() {
            Some(x) => x,
            None => "someone",
        };

        let response = tmpl::apply(
            TEMPLATE,
            &content!(
                "title" => "Hello, world!",
                "name" => name,
                "noun" => "templates",
                "verb" => "create",
                "secret" => &self.secret_code
            ),
        );

        Response::new(Body::from(response))
    }

    fn not_found(&self, path: String, req: Request) -> Response {
        Response::new(Body::from(format!("404 not found: path {}", path)))
    }
}

impl Server for ExampleServer {
    fn respond(&self, path: String, req: Request, _: &str) -> Response {
        if path.starts_with("/static/") {
            return self.serve_static_files(path, "/static/", "/tmp");
        }

        if path.starts_with("/redirect") {
            let mut response = Response::new(Body::from(""));
            self.redirect("http://google.com", &mut response);
            return response;
        }

        match path.as_str() {
            "/" => self.index(path, req),
            _ => self.not_found(path, req),
        }
    }
}

#[tokio::main]
async fn main() {
    let port = flags::define_flag!("port", 9988, "the port to use");
    let secret_code = flags::define_flag!("secret_code", String::new(), "a secret code word");
    flags::parse_flags!(port, secret_code);

    println!("Start server...");
    ws::serve(ExampleServer::new(secret_code.value()), port.value()).await;
}

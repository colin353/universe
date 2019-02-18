#[macro_use]
extern crate tmpl;
extern crate ws;
use ws::{Body, Request, Response, Server};

static MSG: &str = "Start svr: {}";
static TEMPLATE: &str = "Hello, {{name}}!";

#[derive(Copy, Clone)]
struct ExampleServer {}

impl ExampleServer {
    fn new() -> Self {
        ExampleServer {}
    }

    fn index(&self, path: String, req: Request) -> Response {
        let name = match req.uri().query() {
            Some(x) => x,
            None => "someone",
        };

        let response = tmpl::apply(
            TEMPLATE,
            &content!(
                "name" => name
            ),
        );

        Response::new(Body::from(response))
    }

    fn not_found(&self, path: String, req: Request) -> Response {
        Response::new(Body::from(format!("404 not found: path {}", path)))
    }
}

impl Server for ExampleServer {
    fn respond(&self, path: String, req: Request) -> Response {
        match path.as_str() {
            "/" => self.index(path, req),
            _ => self.not_found(path, req),
        }
    }
}

fn main() {
    println!("Start server...");
    ExampleServer::new().serve(8080);
}

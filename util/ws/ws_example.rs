#[macro_use]
extern crate tmpl;
extern crate ws;
use ws::{Body, Request, Response, Server};

static MSG: &str = "Start svr: {}";
static TEMPLATE: &str = include_str!("template.html");

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
                "title" => "Hello, world!",
                "name" => name,
                "noun" => "templates",
                "verb" => "create"
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

fn main() {
    println!("Start server...");
    ExampleServer::new().serve(9988);
}

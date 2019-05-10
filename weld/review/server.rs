#[macro_use]
extern crate tmpl;
extern crate ws;
use ws::{Body, Request, Response, Server};

static MSG: &str = "Start svr: {}";
static TEMPLATE: &str = include_str!("template.html");
static CSS: &str = include_str!("style.css");

#[derive(Copy, Clone)]
pub struct ReviewServer {}

impl ReviewServer {
    pub fn new() -> Self {
        Self {}
    }

    fn index(&self, path: String, req: Request) -> Response {
        let name = match req.uri().query() {
            Some(x) => x,
            None => "someone",
        };

        let response = tmpl::apply(
            TEMPLATE,
            &content!(
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

impl Server for ReviewServer {
    fn respond(&self, path: String, req: Request) -> Response {
        match path.as_str() {
            "/static/style.css" => Response::new(Body::from(CSS)),
            "/" => self.index(path, req),
            _ => self.not_found(path, req),
        }
    }
}

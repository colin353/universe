#[macro_use]
extern crate tmpl;
extern crate ws;
use ws::{Body, Request, Response, Server};

static TEMPLATE: &str = include_str!("template.html");
static INDEX: &str = include_str!("homepage.html");
static CSS: &str = include_str!("style.css");

#[derive(Copy, Clone)]
pub struct ReviewServer {}

impl ReviewServer {
    pub fn new() -> Self {
        Self {}
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        let page = tmpl::apply(
            INDEX,
            &content!(;
                "progress" => vec![
                    content!("title" => "test 123"),
                    content!("title" => "test 345")
                ]
            ),
        );

        Response::new(Body::from(self.wrap_template(page)))
    }

    fn wrap_template(&self, content: String) -> String {
        tmpl::apply(
            TEMPLATE,
            &content!(
                "title" => "weld - review",
                "content" => content),
        )
    }

    fn not_found(&self, path: String, _req: Request) -> Response {
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

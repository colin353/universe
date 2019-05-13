#[macro_use]
extern crate tmpl;
extern crate weld;
extern crate ws;
use ws::{Body, Request, Response, Server};

use weld::WeldServer;

static TEMPLATE: &str = include_str!("template.html");
static INDEX: &str = include_str!("homepage.html");
static CSS: &str = include_str!("style.css");

#[derive(Clone)]
pub struct ReviewServer {
    client: weld::WeldServerClient,
}

mod render {
    pub fn change(c: &weld::Change) -> tmpl::ContentsMap {
        content!(
            "id" => format!("{}", c.get_id()),
            "author" => c.get_author()
        )
    }
}

impl ReviewServer {
    pub fn new(client: weld::WeldServerClient) -> Self {
        Self { client: client }
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        let changes = self.client.list_changes().iter().map(|c| render::change(c)).collect::<Vec<_>>();

        let page = tmpl::apply(
            INDEX,
            &content!(;
                "progress" => changes
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

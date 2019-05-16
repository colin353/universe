#[macro_use]
extern crate tmpl;
extern crate weld;
extern crate ws;
use ws::{Body, Request, Response, Server};

use weld::WeldServer;

mod render;

static TEMPLATE: &str = include_str!("template.html");
static CHANGE: &str = include_str!("change.html");
static INDEX: &str = include_str!("homepage.html");
static CSS: &str = include_str!("style.css");

#[derive(Clone)]
pub struct ReviewServer {
    client: weld::WeldServerClient,
}

impl ReviewServer {
    pub fn new(client: weld::WeldServerClient) -> Self {
        Self { client: client }
    }

    fn wrap_template(&self, content: String) -> String {
        tmpl::apply(
            TEMPLATE,
            &content!(
                "title" => "weld - review",
                "content" => content),
        )
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        let changes = self
            .client
            .list_changes()
            .iter()
            .map(|c| render::change(c))
            .collect::<Vec<_>>();

        let page = tmpl::apply(
            INDEX,
            &content!(;
                "progress" => changes
            ),
        );

        Response::new(Body::from(self.wrap_template(page)))
    }

    fn change(&self, path: String, req: Request) -> Response {
        // The URL will contain a number at the start. Try to extract it.
        let first_component = match path[1..].split("/").next() {
            Some(c) => c,
            None => return self.not_found(path.clone(), req),
        };
        let id = match first_component.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return self.not_found(path.clone(), req),
        };

        let mut request = weld::Change::new();
        request.set_id(id);
        let change = self.client.get_change(request);
        if !change.get_found() {
            return self.not_found(path.clone(), req);
        }

        let page = tmpl::apply(CHANGE, &render::change(&change));

        Response::new(Body::from(self.wrap_template(page)))
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
            _ => self.change(path, req),
        }
    }
}

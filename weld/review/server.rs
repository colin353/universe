#[macro_use]
extern crate tmpl;
extern crate weld;
extern crate ws;
use ws::{Body, Request, Response, Server};

use weld::WeldServer;

mod render;

static MODIFIED_FILES: &str = include_str!("modified_files.html");
static DIFF_VIEW: &str = include_str!("diff_view.html");
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
        let mut path_components = path[1..].split("/");
        let first_component = match path_components.next() {
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

        let maybe_filename = path_components.next();
        if let Some(filename) = maybe_filename {
            return self.change_detail(filename, change, req);
        }

        let mut content = render::change(&change);
        let body = tmpl::apply(MODIFIED_FILES, &content);
        content.insert("body", body);

        let page = tmpl::apply(CHANGE, &content);
        Response::new(Body::from(self.wrap_template(page)))
    }

    fn change_detail(&self, filename: &str, change: weld::Change, req: Request) -> Response {
        let mut found = false;
        let mut content = content!();
        for history in change.get_changes() {
            if history.get_filename() == format!("/{}", filename) {
                found = true;
                content = render::file_history(history);
                break;
            }
        }
        if !found {
            return self.not_found(filename.to_owned(), req);
        }

        let diff_view = tmpl::apply(DIFF_VIEW, &content);

        let mut content = render::change(&change);
        content.insert("body", diff_view);
        let page = tmpl::apply(CHANGE, &content);
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

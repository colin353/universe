#[macro_use]
extern crate tmpl;
extern crate auth_client;
extern crate task_client;
extern crate weld;
extern crate ws;
use ws::{Body, Request, Response, Server};

use auth_client::AuthServer;
use weld::WeldServer;

mod render;

static MODIFIED_FILES: &str = include_str!("modified_files.html");
static DIFF_VIEW: &str = include_str!("diff_view.html");
static TEMPLATE: &str = include_str!("template.html");
static CHANGE: &str = include_str!("change.html");
static INDEX: &str = include_str!("homepage.html");

#[derive(Clone)]
pub struct ReviewServer {
    client: weld::WeldServerClient,
    static_dir: String,
    base_url: String,
    auth: auth_client::AuthClient,
    task_client: task_client::TaskRemoteClient,
}

impl ReviewServer {
    pub fn new(
        client: weld::WeldServerClient,
        static_dir: String,
        base_url: String,
        auth: auth_client::AuthClient,
        task_client: task_client::TaskRemoteClient,
    ) -> Self {
        Self {
            client: client,
            static_dir: static_dir,
            base_url: base_url,
            auth: auth,
            task_client: task_client,
        }
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

        let mut req = weld::GetSubmittedChangesRequest::new();
        req.set_limit(15);
        let submitted_changes = self
            .client
            .get_submitted_changes(req)
            .iter()
            .map(|c| render::change(c))
            .collect::<Vec<_>>();

        let page = tmpl::apply(
            INDEX,
            &content!(;
                "progress" => changes
                "submitted" => submitted_changes
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

        let filename = path_components.collect::<Vec<_>>().join("/");
        if !filename.is_empty() {
            return self.change_detail(&filename, change, req);
        }

        let mut content = render::change(&change);

        if !change.get_task_id().is_empty() {
            if let Some(response) = self.task_client.get_status(change.get_task_id().to_owned()) {
                content.insert(
                    "tasks",
                    tmpl::ContentsMultiMap::from(render::get_task_pills(&response)),
                )
            }
        }

        let body = tmpl::apply(MODIFIED_FILES, &content);
        content.insert("body", body);

        let page = tmpl::apply(CHANGE, &content);
        Response::new(Body::from(self.wrap_template(page)))
    }

    fn change_detail(&self, filename: &str, change: weld::Change, req: Request) -> Response {
        let mut found = false;
        let mut content = content!();

        let maybe_last_snapshot = change
            .get_changes()
            .iter()
            .filter_map(|c| c.get_snapshots().iter().map(|x| x.get_snapshot_id()).max())
            .max();

        let last_snapshot_id = match maybe_last_snapshot {
            Some(x) => x,
            None => return self.not_found(filename.to_owned(), req),
        };

        for history in change.get_changes().iter().filter(|h| {
            h.get_snapshots()
                .iter()
                .filter(|x| x.get_snapshot_id() == last_snapshot_id)
                .next()
                .is_some()
        }) {
            if history.get_filename() == format!("/{}", filename) {
                found = true;
                if let Some(f_content) = render::file_history(history, 0) {
                    content = f_content;
                    content.insert("next_file", "");
                }
            } else if found {
                content.insert("next_file", history.get_filename());
                break;
            }
        }
        if !found {
            return self.not_found(filename.to_owned(), req);
        }

        content.insert("id", change.get_id());
        let diff_view = tmpl::apply(DIFF_VIEW, &content);

        let mut content = render::change(&change);
        content.insert("body", diff_view);
        let page = tmpl::apply(CHANGE, &content);
        Response::new(Body::from(self.wrap_template(page)))
    }

    fn not_found(&self, path: String, _req: Request) -> Response {
        Response::new(Body::from(format!("404 not found: path {}", path)))
    }

    fn start_task(&self, path: String, req: Request) -> Response {
        if path.starts_with("/api/tasks/build/") {
            let change_id: i64 = match path.rsplit("/").next() {
                Some(c) => c.parse().unwrap_or(0),
                None => return Response::new(Body::from("no such change")),
            };

            let mut args = task_client::ArgumentsBuilder::new();
            args.add_int("change_id", change_id);

            let mut response = self
                .task_client
                .create_task(String::from("try_submit"), args.build());

            let mut c = weld::Change::new();
            c.set_id(change_id as u64);
            c.set_task_id(response.take_task_id());
            self.client.update_change_metadata(c);
        }
        Response::new(Body::from("OK"))
    }
}

impl Server for ReviewServer {
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        if path.starts_with("/static/") {
            return self.serve_static_files(path, "/static/", &self.static_dir);
        }

        let result = self.auth.authenticate(token.to_owned());
        if !result.get_success() {
            let challenge = self
                .auth
                .login_then_redirect(format!("{}{}", self.base_url, path));
            let mut response = Response::new(Body::from("redirect to login"));
            self.set_cookie(challenge.get_token(), &mut response);
            self.redirect(challenge.get_url(), &mut response);
            return response;
        }

        if path.starts_with("/api/tasks/") {
            return self.start_task(path, req);
        }

        match path.as_str() {
            "/" => self.index(path, req),
            _ => self.change(path, req),
        }
    }
}

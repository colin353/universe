use std::sync::Arc;
use ws::Server;

extern crate tmpl;
mod render;

static MODIFIED_FILES: &str = include_str!("modified_files.html");
static DIFF_VIEW: &str = include_str!("diff_view.html");
static TEMPLATE: &str = include_str!("template.html");
static CHANGE: &str = include_str!("change.html");
static INDEX: &str = include_str!("homepage.html");

fn fake_change() -> service::Change {
    service::Change {
        id: 123,
        submitted_id: 234,
        description: "Add fake data to the changes list".to_string(),
        status: service::ChangeStatus::Pending,
        repo_name: "example".to_string(),
        repo_owner: "colin".to_string(),
        owner: "colin".to_string(),
    }
}

#[derive(Clone)]
pub struct SrcUIServer {
    client: service::SrcServerAsyncClient,
}

impl SrcUIServer {
    pub fn new(address: String, port: u16) -> Self {
        let connector = Arc::new(bus_rpc::HyperClient::new(address, port));
        Self {
            client: service::SrcServerAsyncClient::new(connector),
        }
    }

    fn wrap_template(&self, content: String) -> String {
        tmpl::apply(
            TEMPLATE,
            &tmpl::content!(
                "title" => "src",
                "content" => content
            ),
        )
    }

    async fn index_result(&self) -> std::io::Result<ws::Response> {
        let mut req = service::ListChangesRequest::new();
        req.owner = "colin".to_string();

        let response = self.client.list_changes(req).await.map_err(|e| {
            // TODO: choose a better error kind
            std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("failed to list changes: {:?}", e),
            )
        })?;

        if response.failed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("server failed: {:?}", response.error_message),
            ));
        }

        let changes = response.changes;

        let mut req = service::ListChangesRequest::new();
        req.limit = 15;
        let submitted_changes = self
            .client
            .list_changes(req)
            .await
            .map_err(|e| {
                // TODO: choose a better error kind
                std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("failed to list changes: {:?}", e),
                )
            })?
            .changes;

        let page = tmpl::apply(
            INDEX,
            &tmpl::content!(;
                "progress" => changes.iter().map(|c| render::change(c)).collect(),
                "submitted" => submitted_changes.iter().map(|c| render::change(c)).collect()
            ),
        );

        Ok(ws::Response::new(ws::Body::from(self.wrap_template(page))))
    }

    async fn show_change(&self, path: String, req: ws::Request) -> ws::Response {
        let mut path_components = path[1..].split("/");
        let repo_owner = match path_components.next() {
            Some(c) => c,
            None => return self.not_found(path.clone()),
        };
        let repo_name = match path_components.next() {
            Some(c) => c,
            None => return self.not_found(path.clone()),
        };
        let third_component = match path_components.next() {
            Some(c) => c,
            None => return self.not_found(path.clone()),
        };
        let id = match third_component.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return self.not_found(path.clone()),
        };

        // TODO: read via client
        let change = fake_change();

        let mut r = service::GetChangeRequest::new();
        r.repo_owner = repo_owner.to_owned();
        r.repo_name = repo_name.to_owned();
        r.id = id;
        let response = match self
            .client
            .get_change(r)
            .await
            .map_err(|e| {
                // TODO: choose a better error kind
                std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("failed to get change: {:?}", e),
                )
            }) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("failed to connect to src service: {:?}", e);
                    return self.failed_500(path.clone());
                }
            };

        if response.failed {
            eprintln!("failed to get change: {}", response.error_message);
            return self.not_found(path.clone());
        }
        let change = response.change;
        let snapshot = response.latest_snapshot;

        let filename = path_components.collect::<Vec<_>>().join("/");
        if !filename.is_empty() {
            return self.change_detail(&filename, change, req);
        }

        let mut content = render::change(&change);
        content.insert("snapshot", render::snapshot(&snapshot));

        let body = tmpl::apply(MODIFIED_FILES, &content);
        content.insert("body", body);

        let page = tmpl::apply(CHANGE, &content);
        ws::Response::new(ws::Body::from(self.wrap_template(page)))
    }

    fn change_detail(
        &self,
        filename: &str,
        change: service::Change,
        req: ws::Request,
    ) -> ws::Response {
        panic!("oh no!");
    }

    async fn index(&self, _path: String, _req: ws::Request) -> ws::Response {
        match self.index_result().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{:?}", e);
                ws::Response::new(ws::Body::from(""))
            }
        }
    }
}

impl ws::Server for SrcUIServer {
    fn respond_future(&self, path: String, req: ws::Request, _: &str) -> ws::ResponseFuture {
        let _self = self.clone();

        if path.starts_with("/static/") {
            return Box::pin(std::future::ready(
                _self.serve_static_files(path, "/static/", "/tmp"),
            ));
        }

        if path.starts_with("/redirect") {
            let mut response = ws::Response::new(ws::Body::from(""));
            _self.redirect("http://google.com", &mut response);
            return Box::pin(std::future::ready(response));
        }

        match path.as_str() {
            "/" => Box::pin(async move { _self.index(path, req).await }),
            _ => Box::pin(async move { _self.show_change(path, req).await }),
        }
    }
}

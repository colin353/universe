use std::sync::Arc;
use ws::Server;

extern crate tmpl;
mod render;

static MODIFIED_FILES: &str = include_str!("modified_files.html");
static DIFF_VIEW: &str = include_str!("diff_view.html");
static TEMPLATE: &str = include_str!("template.html");
static CHANGE: &str = include_str!("change.html");
static INDEX: &str = include_str!("homepage.html");

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
        req.status = service::ChangeStatus::Pending;
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
        req.owner = "colin".to_string();
        req.status = service::ChangeStatus::Submitted;
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

        let mut r = service::GetChangeRequest::new();
        r.repo_owner = repo_owner.to_owned();
        r.repo_name = repo_name.to_owned();
        r.id = id;
        let response = match self.client.get_change(r).await.map_err(|e| {
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
            return self.change_detail(&filename, change, snapshot, req).await;
        }

        let mut content = render::change(&change);
        content.insert("snapshot", render::snapshot(&snapshot));

        let body = tmpl::apply(MODIFIED_FILES, &content);
        content.insert("body", body);

        let page = tmpl::apply(CHANGE, &content);
        ws::Response::new(ws::Body::from(self.wrap_template(page)))
    }

    async fn change_detail(
        &self,
        path: &str,
        change: service::Change,
        snapshot: service::Snapshot,
        req: ws::Request,
    ) -> ws::Response {
        let (fd, next_file) = {
            let mut files_iter = snapshot.files.iter();
            let mut fd = None;
            let mut next_file = "";
            while let Some(f) = files_iter.next() {
                if f.path == path {
                    fd = Some(f);
                    next_file = match files_iter.next() {
                        Some(f) => &f.path,
                        None => "",
                    };
                }
            }

            if fd.is_none() {
                self.not_found(path.to_string());
            }

            (fd.unwrap(), next_file)
        };

        let original = if fd.kind == service::DiffKind::Added {
            Vec::new()
        } else {
            let r = service::GetBlobsByPathRequest {
                basis: snapshot.basis.clone(),
                paths: vec![path.to_string()],
                ..Default::default()
            };
            let mut response = match self.client.get_blobs_by_path(r).await.map_err(|e| {
                // TODO: choose a better error kind
                std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("failed to get original file: {:?}", e),
                )
            }) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("failed to connect to src service: {:?}", e);
                    return self.failed_500(path.to_string());
                }
            };
            if response.failed || response.blobs.len() != 1 {
                eprintln!("failed to get blob for path: {}", response.error_message);
                return self.failed_500(path.to_string());
            }

            std::mem::replace(&mut response.blobs[0].data, Vec::new())
        };

        let modified = match core::apply(fd.as_view(), &original) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("failed to apply patch: {:?}", e);
                return self.failed_500(path.to_string());
            }
        };

        let mut content = render::change(&change);
        content.insert("snapshot", render::snapshot(&snapshot));
        content.insert(
            "file_history",
            render::file_history(&fd, original, modified),
        );
        content.insert("next_file", next_file);

        let body = tmpl::apply(DIFF_VIEW, &content);
        content.insert("body", body);

        let page = tmpl::apply(CHANGE, &content);
        ws::Response::new(ws::Body::from(self.wrap_template(page)))
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

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
    queue: Option<queue_bus::QueueAsyncClient>,
    auth: Option<auth_client::AuthAsyncClient>,
    base_url: String,
}

impl SrcUIServer {
    pub fn new(
        address: String,
        port: u16,
        base_url: String,
        auth: Option<auth_client::AuthAsyncClient>,
        queue: Option<queue_bus::QueueAsyncClient>,
    ) -> Self {
        let connector = Arc::new(bus_rpc::HyperClient::new(address, port));
        Self {
            client: service::SrcServerAsyncClient::new(connector),
            auth,
            base_url,
            queue,
        }
    }

    pub fn new_metal(
        service_name: String,
        base_url: String,
        auth: Option<auth_client::AuthAsyncClient>,
        queue: Option<queue_bus::QueueAsyncClient>,
    ) -> Self {
        let connector = Arc::new(bus_rpc::MetalAsyncClient::new(service_name));
        Self {
            client: service::SrcServerAsyncClient::new(connector),
            auth,
            base_url,
            queue,
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

    async fn api(&self, path: &str, _req: ws::Request, token: String) -> ws::Response {
        let mut path_iter = path.rsplit("/");
        let verb = match path_iter.next() {
            Some(s) => s,
            None => return ws::Response::new(ws::Body::from("no such change")),
        };
        let change_id: u64 = match path_iter.next() {
            Some(c) => c.parse().unwrap_or(0),
            None => return ws::Response::new(ws::Body::from("no such change")),
        };
        let repo_name = match path_iter.next() {
            Some(s) => s,
            None => return ws::Response::new(ws::Body::from("no such change")),
        };
        let repo_owner = match path_iter.next() {
            Some(s) => s,
            None => return ws::Response::new(ws::Body::from("no such change")),
        };

        if verb == "archive" {
            let resp = self
                .client
                .update_change(service::UpdateChangeRequest {
                    token,
                    change: service::Change {
                        id: change_id,
                        status: service::ChangeStatus::Archived,
                        repo_name: repo_name.to_string(),
                        repo_owner: repo_owner.to_string(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .unwrap();

            if resp.failed {
                eprintln!("request failed: {:?}", resp.error_message);
                return ws::Response::new(ws::Body::from("failed"));
            }

            return ws::Response::new(ws::Body::from("ok"));
        } else if verb == "submit" {
            let client = match &self.queue {
                Some(c) => c,
                None => return ws::Response::new(ws::Body::from("unknown api method")),
            };

            if let Ok(_) = client
                .enqueue(queue_bus::EnqueueRequest {
                    queue: "presubmit".to_string(),
                    msg: queue_bus::Message {
                        arguments: vec![queue_bus::Artifact {
                            name: "change".to_string(),
                            value_int: change_id as i64,
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                })
                .await
            {
                ws::Response::new(ws::Body::from("ok"))
            } else {
                ws::Response::new(ws::Body::from("failed to enqueue task"))
            }
        } else {
            ws::Response::new(ws::Body::from("unknown api method"))
        }
    }

    async fn index_result(&self, token: String, username: String) -> std::io::Result<ws::Response> {
        let mut req = service::ListChangesRequest::new();
        req.token = token.clone();
        req.owner = username.clone();
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
        req.token = token;
        req.limit = 15;
        req.owner = username;
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

    async fn show_change(&self, path: String, req: ws::Request, token: String) -> ws::Response {
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
        r.token = token;
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
        _req: ws::Request,
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
        content.insert("path", path.to_string());

        let body = tmpl::apply(DIFF_VIEW, &content);
        content.insert("body", body);

        let page = tmpl::apply(CHANGE, &content);
        ws::Response::new(ws::Body::from(self.wrap_template(page)))
    }

    async fn index(
        &self,
        _path: String,
        _req: ws::Request,
        token: String,
        username: String,
    ) -> ws::Response {
        match self.index_result(token, username).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{:?}", e);
                ws::Response::new(ws::Body::from(""))
            }
        }
    }
}

impl ws::Server for SrcUIServer {
    fn respond_future(&self, path: String, req: ws::Request, token: &str) -> ws::ResponseFuture {
        let _self = self.clone();
        let token = token.to_owned();
        Box::pin(async move {
            let username = if let Some(auth) = _self.auth.as_ref() {
                let result = match auth.authenticate(token.clone()).await {
                    Ok(r) => r,
                    Err(_) => {
                        return _self.failed_500("failed to reach auth service".to_string());
                    }
                };
                if !result.success {
                    let challenge = auth
                        .login_then_redirect(format!("{}{}", _self.base_url, path))
                        .await;
                    let mut response = ws::Response::new(ws::Body::from("redirect to login"));
                    _self.redirect(&challenge.url, &mut response);
                    return response;
                }

                result.username
            } else {
                String::from("colin")
            };

            if path.starts_with("/static/") {
                return _self.serve_static_files(path, "/static/", "/tmp");
            }

            if path.starts_with("/redirect") {
                let mut response = ws::Response::new(ws::Body::from(""));
                _self.redirect("http://google.com", &mut response);
                return response;
            }

            match path.as_str() {
                "/" => _self.index(path, req, token, username).await,
                x if x.starts_with("/api/") => _self.api(&path, req, token).await,
                _ => _self.show_change(path, req, token).await,
            }
        })
    }
}

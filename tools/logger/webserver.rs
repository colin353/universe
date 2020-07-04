#[macro_use]
extern crate tmpl;

use auth_client::AuthServer;
use ws::{Body, Request, Response, Server};

use logger_client::get_timestamp;
use logger_grpc_rust::*;

use std::collections::HashMap;

static TEMPLATE: &str = include_str!("html/template.html");
static TABLE: &str = include_str!("html/table.html");
static JSON: &str = include_str!("html/json.html");
static GRAPH: &str = include_str!("html/graph.html");
static EDIT: &str = include_str!("html/edit.html");

#[derive(Clone)]
pub struct LoggerWebServer {
    handler: server_lib::LoggerServiceHandler,
    auth: auth_client::AuthClient,
    base_url: String,
}

impl LoggerWebServer {
    pub fn new(
        handler: server_lib::LoggerServiceHandler,
        auth: auth_client::AuthClient,
        base_url: String,
    ) -> Self {
        Self {
            auth,
            handler,
            base_url,
        }
    }

    fn wrap_template(&self, content: String) -> String {
        tmpl::apply(
            TEMPLATE,
            &content!(
                "content" => content
            ),
        )
    }

    fn graph(&self) -> Response {
        Response::new(Body::from(GRAPH.to_string()))
    }

    fn edit(&self) -> Response {
        Response::new(Body::from(EDIT.to_string()))
    }

    fn index(
        &self,
        request: Request,
        log_name: &str,
        renderer: &str,
        extractor_name: &str,
    ) -> Response {
        let mut end_time = get_timestamp();
        let mut start_time = end_time - 86400;

        // Determine filters passed by query
        let query = match request.uri().query() {
            Some(q) => q,
            None => "",
        };

        let mut query_params = ws_utils::parse_params(query);

        if let Some(start) = query_params.get("start_time") {
            start_time = start.parse().unwrap();
        }

        if let Some(end) = query_params.get("end_time") {
            end_time = end.parse().unwrap();
        }

        let mut req = GetLogsRequest::new();
        req.set_log(log_processing::string_to_log(log_name));
        req.set_start_time(start_time);
        req.set_end_time(end_time);
        let response = self.handler.get_logs(req);

        let extractors = match log_processing::EXTRACTORS.get(log_name) {
            Some(x) => x,
            None => {
                return Response::new(Body::from(format!("unknown log_name: {}", log_name)));
            }
        };

        let extractor = match extractors.iter().find(|(name, f)| name == &extractor_name) {
            Some((name, f)) => f,
            None => {
                return Response::new(Body::from(format!("unknown extractor: {}", extractor_name)));
            }
        };

        let available_filters = match log_processing::FILTERS.get(log_name) {
            Some(x) => x,
            None => {
                return Response::new(Body::from(format!("unknown log_name: {}", log_name)));
            }
        };

        let filters = match query_params.remove("filters") {
            Some(x) => {
                let mut filters = Vec::new();
                for f in x.split(",") {
                    match available_filters.iter().find(|(fname, filter)| &f == fname) {
                        Some((_, filter)) => filters.push(filter),
                        None => {
                            return Response::new(Body::from(format!("unknown filter: {}", f)));
                        }
                    }
                }
                filters
            }
            None => Vec::new(),
        };

        let args = HashMap::new();
        let output: Vec<(u64, String)> = response
            .get_messages()
            .iter()
            .filter(|x| {
                for filter in &filters {
                    if !filter(&query_params, x) {
                        return false;
                    }
                }
                true
            })
            .map(|m| extractor(&args, m))
            .take(1000000)
            .collect();

        let body = match renderer {
            "table" => tmpl::apply(
                TABLE,
                &content!(
                    "data" => output.into_iter().map(|(_, d)| d).collect::<Vec<_>>().join("")
                ),
            ),
            "json" => {
                return Response::new(Body::from(tmpl::apply(
                    JSON,
                    &content!(
                        "data" => output.iter().map(|(t, d)| format!("[{}, {}]", t, d)).collect::<Vec<_>>().join(",")
                    ),
                )))
            }
            _ => {
                return Response::new(Body::from(format!("unknown renderer: {}", renderer)));
            }
        };

        Response::new(Body::from(self.wrap_template(body)))
    }
}

impl Server for LoggerWebServer {
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        let result = self.auth.authenticate(token.to_owned());
        if !result.get_success() {
            let challenge = self
                .auth
                .login_then_redirect(format!("{}{}", self.base_url, path));
            let mut response = Response::new(Body::from("redirect to login"));
            self.redirect(challenge.get_url(), &mut response);
            return response;
        }

        let mut path_split = path.split("/");
        path_split.next(); // drop leading /

        let first_component = match path_split.next() {
            Some(x) => x,
            None => return self.not_found(path),
        };

        if first_component == "graph" {
            return self.graph();
        } else if first_component == "edit" {
            return self.edit();
        }

        let log_name = first_component;

        let renderer = match path_split.next() {
            Some(x) => x,
            None => return self.not_found(path),
        };
        let extractor = match path_split.next() {
            Some(x) => x,
            None => return self.not_found(path),
        };

        return self.index(req, log_name, renderer, extractor);
    }
}

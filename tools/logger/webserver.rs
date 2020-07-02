#[macro_use]
extern crate tmpl;

use auth_client::AuthServer;
use ws::{Body, Request, Response, Server};

use logger_client::get_timestamp;
use logger_grpc_rust::*;

use std::collections::HashMap;

static TEMPLATE: &str = include_str!("html/template.html");
static TABLE: &str = include_str!("html/table.html");

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

    fn index(&self, log_name: &str, renderer: &str, extractor_name: &str) -> Response {
        let end_time = get_timestamp();
        let start_time = end_time - 2000;

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

        let args = HashMap::new();
        let output: Vec<String> = response
            .get_messages()
            .iter()
            .map(|m| extractor(&args, m))
            .collect();

        let body = match renderer {
            "table" => tmpl::apply(
                TABLE,
                &content!(
                    "data" => output.join("")
                ),
            ),
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

        let log_name = match path_split.next() {
            Some(x) => x,
            None => return self.not_found(path),
        };
        let renderer = match path_split.next() {
            Some(x) => x,
            None => return self.not_found(path),
        };
        let extractor = match path_split.next() {
            Some(x) => x,
            None => return self.not_found(path),
        };

        return self.index(log_name, renderer, extractor);
    }
}

#[macro_use]
extern crate flags;
extern crate auth_client;
extern crate ws;
use ws::{Body, Request, Response, Server};

#[derive(Clone)]
pub struct HomepageServer {
    static_dir: String,
    base_url: String,
}
impl HomepageServer {
    fn new(static_dir: String, base_url: String) -> Self {
        Self {
            static_dir: static_dir,
            base_url: base_url,
        }
    }
}

impl Server for HomepageServer {
    fn respond(&self, path: String, _req: Request, token: &str) -> Response {
        if path == "/" || path.is_empty() {
            return self.serve_static_files("/index.html".to_string(), "", &self.static_dir);
        }

        return self.serve_static_files(path, "", &self.static_dir);
    }
}

#[tokio::main]
async fn main() {
    let port = define_flag!("port", 8080, "the port to bind to");
    let base_url = define_flag!(
        "base_url",
        "https://colinmerkel.xyz".to_string(),
        "the base URL of the website"
    );
    let static_files = define_flag!(
        "static_files",
        String::from("/static/"),
        "the directory containing static files"
    );
    parse_flags!(port, base_url, static_files);

    ws::serve(
        HomepageServer::new(static_files.value(), base_url.value()),
        port.value(),
    )
    .await;
}

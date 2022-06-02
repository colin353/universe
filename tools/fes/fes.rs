#[macro_use]
extern crate flags;
use ws::{Body, Request, Response, Server};

#[derive(Clone)]
struct FrontendServer {
    base_dir: String,
}

impl FrontendServer {
    fn new(base_dir: String) -> Self {
        Self { base_dir }
    }
}

impl Server for FrontendServer {
    fn respond(&self, path: String, req: Request, _: &str) -> Response {
        match path.as_str() {
            _ => self.serve_static_files(path, "", &self.base_dir),
        }
    }
}

#[tokio::main]
async fn main() {
    let port = define_flag!("port", 5464, "the port to serve from");
    let base_dir = define_flag!(
        "base_dir",
        String::from("."),
        "the dir to serve static assets from"
    );
    parse_flags!(base_dir, port);

    println!("Serving at http://localhost:{}", port.value());
    ws::serve(FrontendServer::new(base_dir.value()), port.value()).await;
}

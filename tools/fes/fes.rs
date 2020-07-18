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

    fn index(&self) -> Response {
        let f = std::fs::read_to_string(format!("{}/index.html", self.base_dir)).unwrap();
        Response::new(Body::from(f))
    }
}

impl Server for FrontendServer {
    fn respond(&self, path: String, req: Request, _: &str) -> Response {
        match path.as_str() {
            "/" => self.index(),
            _ => self.serve_static_files(path, "", &self.base_dir),
        }
    }
}

fn main() {
    let port = define_flag!("port", 5464, "the port to serve from");
    let base_dir = define_flag!(
        "base_dir",
        String::new(),
        "the dir to serve static assets from"
    );
    parse_flags!(base_dir, port);

    println!("Serving at http://localhost:{}", port.value());
    FrontendServer::new(base_dir.value()).serve(port.value());
}

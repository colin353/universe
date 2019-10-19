#[macro_use]
extern crate flags;
extern crate ws;
use ws::{Body, Request, Response, Server};

#[derive(Clone)]
pub struct HomepageServer {
    static_dir: String,
}
impl HomepageServer {
    fn new(static_dir: String) -> Self {
        Self {
            static_dir: static_dir,
        }
    }
}
impl Server for HomepageServer {
    fn respond(&self, path: String, req: Request) -> Response {
        if path.starts_with("/static/") {
            return self.serve_static_files(path, "/static/", &self.static_dir);
        }

        Response::new(Body::from("hello world!"))
    }
}

fn main() {
    let port = define_flag!("port", 8080, "the port to bind to");
    let static_files = define_flag!(
        "static_files",
        String::from("/static/"),
        "the directory containing static files"
    );
    parse_flags!(port, static_files);

    HomepageServer::new(static_files.value()).serve(port.value());
}

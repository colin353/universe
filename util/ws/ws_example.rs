extern crate tmpl;
extern crate ws;
use ws::Server;

static MSG: &str = "Start svr: {}";
static TEMPLATE: &str = "Hello, {{world}}!";

#[derive(Copy, Clone)]
struct ExampleServer {}

impl ExampleServer {
    fn new() -> Self {
        ExampleServer {}
    }

    fn index(&self, path: String, req: ws::Request) -> ws::Response {
        ws::Response::new(ws::Body::from("hi, index"))
    }

    fn not_found(&self, path: String, req: ws::Request) -> ws::Response {
        ws::Response::new(ws::Body::from(format!("404 not found: path {}", path)))
    }
}

impl ws::Server for ExampleServer {
    fn respond(&self, path: String, req: ws::Request) -> ws::Response {
        match path.as_str() {
            "/" => self.index(path, req),
            _ => self.not_found(path, req),
        }
    }
}

fn main() {
    println!("Start server...");
    ExampleServer::new().serve(8080);
}

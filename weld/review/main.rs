#[macro_use]
extern crate tmpl;
extern crate server;
extern crate ws;
use ws::{Body, Request, Response, Server};

fn main() {
    server::ReviewServer::new().serve(8080);
    println!("Starting server...");
}

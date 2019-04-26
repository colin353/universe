#[macro_use]
extern crate tmpl;
extern crate server;
extern crate ws;
use ws::Server;

fn main() {
    println!("Starting server...");
    server::ReviewServer::new().serve(8080);
}

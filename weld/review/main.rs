#[macro_use]
extern crate flags;
extern crate server;
extern crate ws;
extern crate weld;

use std::fs::File;
use std::io::Read;

use ws::Server;

fn main() {
    println!("Starting server...");

    let server_hostname = define_flag!(
        "server_hostname",
        String::from("localhost:8001"),
        "the hostname for the remote weld service"
    );
    let server_port = define_flag!("server_port", 8001, "the port to connect to");
    let use_tls = define_flag!("use_tls", true, "Whether or not to use TLS encryption");
    let tls_hostname = define_flag!(
        "tls_hostname",
        String::from("server.weld.io"),
        "the hostname to require the server to authenticate itself as"
    );
    let root_ca = define_flag!(
        "root_ca",
        String::from(""),
        "path to a file containing the root CA .der file"
    );
    let cert = define_flag!(
        "cert",
        String::from(""),
        "path to a file containing the client cert .der file"
    );
    let port = define_flag!(
        "port",
        8080,
        "the port to bind to");

    parse_flags!(
        server_hostname,
        server_port,
        use_tls,
        tls_hostname,
        root_ca,
        cert,
        port
    );

    let client = if use_tls.value() {
        let mut root_ca_contents = Vec::new();
        File::open(root_ca.value())
            .unwrap()
            .read_to_end(&mut root_ca_contents)
            .unwrap();
        let mut cert_contents = Vec::new();
        File::open(cert.value())
            .unwrap()
            .read_to_end(&mut cert_contents)
            .unwrap();
        weld::WeldServerClient::new_tls(
            &server_hostname.value(),
            &tls_hostname.value(),
            String::from(""),
            server_port.value(),
            root_ca_contents,
            cert_contents,
        )
    } else {
        weld::WeldServerClient::new(
            &server_hostname.value(),
            String::from(""),
            server_port.value(),
        )
    };

    server::ReviewServer::new(client).serve(port.value());
}

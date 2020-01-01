#[macro_use]
extern crate flags;
extern crate auth_client;
extern crate server;
extern crate task_client;
extern crate weld;
extern crate ws;

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
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth.colinmerkel.xyz"),
        "the hostname for auth service"
    );
    let auth_port = define_flag!("auth_port", 8888, "the port for auth service");
    let cert = define_flag!(
        "cert",
        String::from(""),
        "path to a file containing the client cert .der file"
    );
    let port = define_flag!("port", 8080, "the port to bind to");
    let static_files = define_flag!(
        "static_files",
        String::from("/static/"),
        "the directory containing static files"
    );
    let base_url = define_flag!(
        "base_url",
        String::from("http://review.colinmerkel.xyz"),
        "the base URL of the site"
    );
    let task_hostname = define_flag!(
        "task_hostname",
        String::from("localhost"),
        "the hostname of the task service"
    );
    let task_port = define_flag!("task_port", 7777, "the port of the task service");

    parse_flags!(
        server_hostname,
        server_port,
        auth_hostname,
        auth_port,
        use_tls,
        tls_hostname,
        root_ca,
        cert,
        port,
        static_files,
        base_url,
        task_hostname,
        task_port
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

    let task = task_client::TaskRemoteClient::new(task_hostname.value(), task_port.value());
    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());
    server::ReviewServer::new(client, static_files.value(), base_url.value(), auth, task)
        .serve(port.value());
}

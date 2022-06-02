#[macro_use]
extern crate flags;
extern crate auth_client;
extern crate queue_client;
extern crate server;
extern crate weld;
extern crate ws;

use std::fs::File;
use std::io::Read;

use ws::Server;

#[tokio::main]
async fn main() {
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

    let queue_hostname = define_flag!(
        "queue_hostname",
        String::from("queue"),
        "the hostname of the queue service"
    );
    let queue_port = define_flag!("queue_port", 5554, "the port of the queue service");
    let disable_auth = define_flag!("disable_auth", false, "whether to disable auth");
    let auth_token = define_flag!(
        "auth_token",
        String::new(),
        "auth token to use when connecting to weld service"
    );

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
        task_port,
        queue_hostname,
        queue_port,
        disable_auth,
        auth_token
    );

    let client = weld::WeldServerClient::new(
        &server_hostname.value(),
        String::from(""),
        server_port.value(),
    );
    client.set_permanent_token(auth_token.value());

    let queue = queue_client::QueueClient::new(&queue_hostname.value(), queue_port.value());

    let auth = if disable_auth.value() {
        auth_client::AuthClient::new_fake()
    } else {
        auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value())
    };

    ws::serve(
        server::ReviewServer::new(client, static_files.value(), base_url.value(), auth, queue),
        port.value(),
    )
    .await;
}

#[macro_use]
extern crate flags;
#[macro_use]
extern crate tmpl;

use std::sync::Arc;
use ws::Server;

mod render;
mod webserver;

fn main() {
    let web_port = define_flag!("web_port", 9898, "The port to bind to (for web)");
    let grpc_port = define_flag!("grpc_port", 9899, "The port to bind to (for grpc)");
    let index_dir = define_flag!(
        "index_dir",
        String::new(),
        "The directory of the search index."
    );
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth.colinmerkel.xyz"),
        "the hostname for auth service"
    );
    let disable_auth = define_flag!("disable_auth", false, "whether to enable or disable auth");

    let auth_port = define_flag!("auth_port", 8888, "the port for auth service");
    let static_files = define_flag!(
        "static_files",
        String::from("/static/"),
        "the directory containing static files"
    );
    let base_url = define_flag!(
        "base_url",
        String::from("http://localhost:9898"),
        "the base URL of the site"
    );

    parse_flags!(
        web_port,
        grpc_port,
        index_dir,
        auth_hostname,
        auth_port,
        disable_auth,
        static_files,
        base_url
    );

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());
    server.http.set_cpu_pool_threads(2);

    let searcher = Arc::new(search_lib::Searcher::new(&index_dir.path()));

    let auth = match disable_auth.value() {
        true => auth_client::AuthClient::new_fake(),
        false => auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value()),
    };

    let handler = server_lib::SearchServiceHandler::new(searcher.clone(), auth.clone());
    server.add_service(search_grpc_rust::SearchServiceServer::new_service_def(
        handler,
    ));
    let _server = server.build().unwrap();

    webserver::SearchWebserver::new(searcher, static_files.value(), base_url.value(), auth)
        .serve(web_port.value());
}

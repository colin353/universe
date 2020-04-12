extern crate grpc;
extern crate tls_api_native_tls;
extern crate ws;
#[macro_use]
extern crate flags;
extern crate auth_grpc_rust;
extern crate auth_service_impl;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::{Arc, RwLock};
use ws::Server;

fn main() {
    let grpc_port = define_flag!("grpc_port", 8888, "The gRPC port to bind to.");
    let web_port = define_flag!("web_port", 8899, "The web port to bind to.");
    let oauth_client_id = define_flag!("oauth_client_id", String::new(), "The oauth client ID");
    let oauth_client_secret = define_flag!(
        "oauth_client_secret",
        String::new(),
        "The oauth client secret"
    );
    let hostname = define_flag!(
        "hostname",
        String::new(),
        "the publicly accessible hostname"
    );
    let allowed_emails = define_flag!(
        "allowed_emails",
        String::new(),
        "a list of allowed emails separated by commas"
    );
    let cookie_domain = define_flag!(
        "cookie_domain",
        String::from("colinmerkel.xyz"),
        "the domain setting to use for cookies"
    );
    parse_flags!(
        allowed_emails,
        grpc_port,
        web_port,
        hostname,
        cookie_domain,
        oauth_client_id,
        oauth_client_secret
    );

    let email_whitelist = std::collections::HashSet::from_iter(
        allowed_emails.value().split(",").map(|x| x.to_owned()),
    );

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());
    server.http.set_cpu_pool_threads(2);

    let tokens = Arc::new(RwLock::new(HashMap::new()));
    let handler = auth_service_impl::AuthServiceHandler::new(
        hostname.value(),
        oauth_client_id.value(),
        tokens.clone(),
    );

    server.add_service(auth_grpc_rust::AuthenticationServiceServer::new_service_def(handler));

    let _server = server.build().expect("server");
    auth_service_impl::AuthWebServer::new(
        tokens,
        hostname.value(),
        cookie_domain.value(),
        oauth_client_id.value(),
        oauth_client_secret.value(),
        Arc::new(email_whitelist),
    )
    .serve(web_port.value());
}

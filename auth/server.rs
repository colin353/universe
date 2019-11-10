extern crate grpc;
extern crate tls_api_native_tls;
extern crate ws;
#[macro_use]
extern crate flags;
extern crate auth_grpc_rust;
extern crate auth_service_impl;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ws::Server;

fn main() {
    let grpc_port = define_flag!("port", 8888, "The gRPC port to bind to.");
    let web_port = define_flag!("port", 8899, "The web port to bind to.");
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
    parse_flags!(
        grpc_port,
        web_port,
        hostname,
        oauth_client_id,
        oauth_client_secret
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
        oauth_client_id.value(),
        oauth_client_secret.value(),
    )
    .serve(web_port.value());
}

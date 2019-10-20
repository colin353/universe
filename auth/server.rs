extern crate grpc;
extern crate tls_api_native_tls;

#[macro_use]
extern crate flags;
extern crate auth_grpc_rust;
extern crate auth_service_impl;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

fn main() {
    let port = define_flag!("port", 8888, "The port to bind to.");
    let oauth_client_id = define_flag!("oauth_client_id", String::new(), "The oauth client ID");
    let hostname = define_flag!(
        "hostname",
        String::new(),
        "the publicly accessible hostname"
    );
    parse_flags!(port, hostname, oauth_client_id);

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.http.set_cpu_pool_threads(8);

    let tokens = Arc::new(RwLock::new(HashMap::new()));
    let handler = auth_service_impl::AuthServiceHandler::new(
        hostname.value(),
        oauth_client_id.value(),
        tokens,
    );

    server.add_service(auth_grpc_rust::AuthenticationServiceServer::new_service_def(handler));

    let _server = server.build().expect("server");
    loop {
        std::thread::park();
    }
}

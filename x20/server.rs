extern crate grpc;
extern crate tls_api_stub;
extern crate x20_server_lib;

#[macro_use]
extern crate flags;
extern crate x20_grpc_rust as x20;

fn main() {
    let port = define_flag!("port", 8001, "The port to bind to.");
    let largetable_hostname = define_flag!(
        "largetable_hostname",
        String::from("127.0.0.1"),
        "Hostname of the largetable service"
    );
    let largetable_port = define_flag!(
        "largetable_port",
        50051,
        "Which port to connect to on the largetable service"
    );
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth.colinmerkel.xyz"),
        "the hostname for auth service"
    );
    let auth_port = define_flag!("auth_port", 8888, "the port for auth service");
    parse_flags!(
        port,
        largetable_hostname,
        largetable_port,
        auth_hostname,
        auth_port
    );

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.http.set_cpu_pool_threads(2);

    let database = largetable_client::LargeTableRemoteClient::new(
        &largetable_hostname.value(),
        largetable_port.value(),
    );
    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());
    let handler = x20_server_lib::X20ServiceHandler::new(database, auth);
    server.add_service(x20::X20ServiceServer::new_service_def(handler));
    let _server = server.build().expect("server");

    loop {
        std::thread::park();
    }
}

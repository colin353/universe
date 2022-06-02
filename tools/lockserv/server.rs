#[macro_use]
extern crate flags;

fn main() {
    let port = define_flag!("port", 5555, "The gRPC port to bind to");
    parse_flags!(port);

    let handler = server_lib::LockServiceHandler::new();

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(lockserv_grpc_rust::LockServiceServer::new_service_def(
        handler,
    ));
    let _server = server.build().expect("server");

    loop {
        std::thread::park();
    }
}

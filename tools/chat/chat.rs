#[macro_use]
extern crate flags;

fn main() {
    let grpc_port = define_flag!("grpc_port", 6668, "The gRPC port to bind to");
    parse_flags!(grpc_port);

    let handler = chat_service::ChatServiceHandler::new();

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());
    server.http.set_cpu_pool_threads(2);
    server.add_service(chat_grpc_rust::ChatServiceServer::new_service_def(handler));
    let _server = server.build().unwrap();

    loop {
        std::thread::park();
    }
}

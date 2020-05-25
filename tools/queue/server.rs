#[macro_use]
extern crate flags;

fn main() {
    let port = define_flag!("port", 5554, "The gRPC port to bind to");
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
    parse_flags!(port, largetable_port, largetable_hostname);

    let database = largetable_client::LargeTableRemoteClient::new(
        &largetable_hostname.value(),
        largetable_port.value(),
    );
    let handler = server_lib::QueueServiceHandler::new(database);

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.http.set_cpu_pool_threads(2);
    server.add_service(queue_grpc_rust::QueueServiceServer::new_service_def(
        handler,
    ));
    let _server = server.build().expect("server");

    loop {
        std::thread::park();
    }
}

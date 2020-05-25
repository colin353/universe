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
    let lockserv_hostname = define_flag!(
        "lockserv_hostname",
        String::from("127.0.0.1"),
        "Hostname of the lockserv client"
    );
    let lockserv_port = define_flag!("lockserv_port", 5555, "Port of the lockserv service");
    parse_flags!(
        port,
        largetable_port,
        largetable_hostname,
        lockserv_hostname,
        lockserv_port
    );

    let ls =
        lockserv_client::LockservClient::new(&lockserv_hostname.value(), lockserv_port.value());

    let database = largetable_client::LargeTableRemoteClient::new(
        &largetable_hostname.value(),
        largetable_port.value(),
    );
    let handler = server_lib::QueueServiceHandler::new(database, ls);

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.http.set_cpu_pool_threads(2);
    server.add_service(queue_grpc_rust::QueueServiceServer::new_service_def(
        handler.clone(),
    ));
    let _server = server.build().expect("server");

    loop {
        handler.bump();
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

extern crate grpc;

#[macro_use]
extern crate flags;
extern crate largetable_test;
extern crate server_impl;

fn main() {
    let port = define_flag!("port", 7777, "The port to bind to.");
    parse_flags!(port);

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.http.set_cpu_pool_threads(8);

    let database = largetable_test::LargeTableMockClient::new();
    let handler = server_impl::TaskServiceHandler::new(database);
    server.add_service(tasks_grpc_rust::TaskServiceServer::new_service_def(handler));

    let _server = server.build().expect("server");
    loop {
        std::thread::park();
    }
}

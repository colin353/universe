#[macro_use]
extern crate flags;

fn main() {
    let port = define_flag!("port", 3232 as u16, "The port to bind to.");
    let data_dir = define_flag!("data_dir", String::new(), "The data directory to use");
    parse_flags!(port, data_dir);

    let mut handler = server_lib::LoggerServiceHandler::new(data_dir.value());
    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(logger_grpc_rust::LoggerServiceServer::new_service_def(
        handler.clone(),
    ));
    server.http.set_cpu_pool_threads(2);

    let _server = server.build().expect("server");

    loop {
        std::thread::park();
    }
}

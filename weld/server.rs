extern crate grpc;

#[macro_use]
extern crate flags;
extern crate largetable_test;
extern crate weld;
extern crate weld_repo;

extern crate weld_server_lib;

fn main() {
    let port = define_flag!("port", 8001, "The port to bind to.");
    let cert = define_flag!(
        "cert",
        String::from(""),
        "Where to look up the root CA cert"
    );
    let key = define_flag!(
        "key",
        String::from(""),
        "Where to look up the root CA private key"
    );
    parse_flags!(port, cert, key);

    let database = largetable_test::LargeTableMockClient::new();
    let handler = weld_server_lib::WeldServiceHandler::new(database);

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(weld::WeldServiceServer::new_service_def(handler.clone()));
    server.http.set_cpu_pool_threads(4);

    let _server = server.build().expect("server");

    // Wait until closed.
    loop {
        std::thread::park();
    }
}

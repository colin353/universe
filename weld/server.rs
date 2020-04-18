extern crate grpc;

#[macro_use]
extern crate flags;
extern crate largetable_client;
extern crate largetable_test;
extern crate native_tls;
extern crate openssl;
extern crate tls_api;
extern crate tls_api_openssl;
extern crate weld;
extern crate weld_repo;

extern crate weld_server_lib;

use std::fs::File;
use std::io::Read;
use tls_api::TlsAcceptorBuilder;

fn main() {
    let port = define_flag!("port", 8001, "The port to bind to.");
    let root_cert = define_flag!(
        "root_cert",
        String::from(""),
        "Where to look up the root CA cert"
    );
    let pkcs12 = define_flag!(
        "pkcs12",
        String::from(""),
        "Where to look up the server cert (pkcs12)"
    );
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
    let use_mock_largetable = define_flag!(
        "use_mock_largetable",
        false,
        "Whether to use in-memory mock largetable or RPC"
    );
    let use_tls = define_flag!("use_tls", true, "Whether or not to use TLS encryption");
    parse_flags!(
        port,
        pkcs12,
        largetable_hostname,
        largetable_port,
        use_mock_largetable,
        use_tls,
        root_cert
    );

    let mut server = grpc::ServerBuilder::<tls_api_openssl::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.http.set_cpu_pool_threads(8);

    if use_tls.value() {
        let mut p12_contents = Vec::new();
        File::open(pkcs12.value())
            .unwrap()
            .read_to_end(&mut p12_contents)
            .unwrap();

        println!("Read {} bytes of pkcs12", p12_contents.len());

        let acceptor =
            tls_api_openssl::TlsAcceptorBuilder::from_pkcs12(&p12_contents, "test").unwrap();

        server.http.set_tls(acceptor.build().unwrap());
    }

    if use_mock_largetable.value() {
        println!("Using in-memory mock largetable...");
        let database = largetable_test::LargeTableMockClient::new();
        let handler = weld_server_lib::WeldServiceHandler::new(database);

        server.add_service(weld::WeldServiceServer::new_service_def(handler));
    } else {
        let database = largetable_client::LargeTableRemoteClient::new(
            &largetable_hostname.value(),
            largetable_port.value(),
        );
        let handler = weld_server_lib::WeldServiceHandler::new(database);

        server.add_service(weld::WeldServiceServer::new_service_def(handler));
    }

    let _server = server.build().expect("server");

    // Wait until closed.
    loop {
        std::thread::park();
    }
}

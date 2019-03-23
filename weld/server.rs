extern crate grpc;

#[macro_use]
extern crate flags;
extern crate largetable_client;
extern crate largetable_test;
extern crate tls_api;
extern crate tls_api_native_tls;
extern crate weld;
extern crate weld_repo;

extern crate weld_server_lib;

use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use tls_api::TlsAcceptorBuilder;

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
        cert,
        key,
        largetable_hostname,
        largetable_port,
        use_mock_largetable,
        use_tls
    );

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.http.set_cpu_pool_threads(4);

    if use_tls.value() {
        let mut cert_contents = String::new();
        File::open(cert.value())
            .unwrap()
            .read_to_string(&mut cert_contents)
            .unwrap();

        let mut key_contents = String::new();
        File::open(key.value())
            .unwrap()
            .read_to_string(&mut key_contents)
            .unwrap();

        let acceptor = tls_api_native_tls::TlsAcceptorBuilder::from_pkcs12(
            cert_contents.as_bytes(),
            &key_contents,
        )
        .unwrap();

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
        let handler = weld_server_lib::WeldServiceHandler::new(Arc::new(database));

        server.add_service(weld::WeldServiceServer::new_service_def(handler));
    }

    let _server = server.build().expect("server");

    // Wait until closed.
    loop {
        std::thread::park();
    }
}

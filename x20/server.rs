extern crate grpc;
extern crate tls_api_stub;
extern crate x20_server_lib;

#[macro_use]
extern crate flags;
extern crate x20_grpc_rust as x20;

use queue_client::Consumer;

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
    let lockserv_hostname = define_flag!(
        "lockserv_hostname",
        String::from("lockserv"),
        "the hostname of the lock service"
    );
    let lockserv_port = define_flag!("lockserv_port", 5555, "the hostname of the lock service");
    let queue_hostname = define_flag!(
        "queue_hostname",
        String::from("queue"),
        "the hostname of the queue service"
    );
    let queue_port = define_flag!("queue_port", 5554, "the port of the queue service");
    parse_flags!(
        port,
        largetable_hostname,
        largetable_port,
        auth_hostname,
        auth_port,
        lockserv_hostname,
        lockserv_port,
        queue_hostname,
        queue_port
    );

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());

    let database = largetable_client::LargeTableRemoteClient::new(
        &largetable_hostname.value(),
        largetable_port.value(),
    );
    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());
    let handler = x20_server_lib::X20ServiceHandler::new(database, auth);
    server.add_service(x20::X20ServiceServer::new_service_def(handler.clone()));
    let _server = server.build().expect("server");

    let lockserv_client =
        lockserv_client::LockservClient::new(&lockserv_hostname.value(), lockserv_port.value());
    let queue_client = queue_client::QueueClient::new(&queue_hostname.value(), queue_port.value());
    let consumer = consumer::X20Consumer::new(queue_client, lockserv_client, handler);
    std::thread::spawn(move || {
        consumer.start(String::from("publish"));
    });

    loop {
        std::thread::park();
    }
}

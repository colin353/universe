#[macro_use]
extern crate flags;

use largetable_client::LargeTableClient;
use ws::Server;

#[tokio::main]
async fn main() {
    let grpc_port = define_flag!("grpc_port", 5554, "The gRPC port to bind to");
    let web_port = define_flag!("web_port", 5553, "The webserver port to bind to");
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
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth.colinmerkel.xyz"),
        "the hostname of the auth service"
    );
    let auth_port = define_flag!("auth_port", 8888, "the port of the auth service");
    let base_url = define_flag!(
        "base_url",
        String::from("http://tasks.local.colinmerkel.xyz:5553"),
        "the base URL of the queue webservice"
    );
    let use_tls = define_flag!(
        "use_tls",
        false,
        "whether or not to use TLS when connecting to auth"
    );

    parse_flags!(
        grpc_port,
        web_port,
        largetable_port,
        largetable_hostname,
        lockserv_hostname,
        lockserv_port,
        auth_hostname,
        auth_port,
        base_url,
        use_tls
    );

    let ls =
        lockserv_client::LockservClient::new(&lockserv_hostname.value(), lockserv_port.value());

    let database = largetable_client::LargeTableRemoteClient::new(
        &largetable_hostname.value(),
        largetable_port.value(),
    );
    database.wait_for_connection();

    let handler = server_lib::QueueServiceHandler::new(database.clone(), ls, base_url.value());

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());
    server.add_service(queue_grpc_rust::QueueServiceServer::new_service_def(
        handler.clone(),
    ));
    let _server = server.build().expect("server");

    let auth = if use_tls.value() {
        auth_client::AuthClient::new_tls(&auth_hostname.value(), auth_port.value())
    } else {
        auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value())
    };

    std::thread::spawn(move || loop {
        handler.bump();
        std::thread::sleep(std::time::Duration::from_secs(10));
    });

    ws::serve(
        webserver::QueueWebServer::new(database, auth, base_url.value()),
        web_port.value(),
    )
    .await;
}

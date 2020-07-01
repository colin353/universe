#[macro_use]
extern crate flags;

fn main() {
    let port = define_flag!("port", 3232 as u16, "The port to bind to.");
    let data_dir = define_flag!("data_dir", String::new(), "The data directory to use");
    let secret_key = define_flag!("secret_key", String::new(), "The auth secret key");

    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth"),
        "The auth service hostname"
    );
    let auth_port = define_flag!("auth_port", 8888, "The auth service port");
    parse_flags!(port, data_dir, secret_key, auth_hostname, auth_port);

    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());
    auth.global_init(secret_key.value());

    let mut handler = server_lib::LoggerServiceHandler::new(data_dir.value());
    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(logger_grpc_rust::LoggerServiceServer::new_service_def(
        handler.clone(),
    ));
    server.http.set_cpu_pool_threads(2);

    std::thread::spawn(move || {
        handler.reorganize();
        std::thread::sleep(std::time::Duration::from_secs(300));
    });

    let _server = server.build().expect("server");

    loop {
        std::thread::park();
    }
}

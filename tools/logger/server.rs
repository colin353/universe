#[macro_use]
extern crate flags;

#[tokio::main]
async fn main() {
    let grpc_port = define_flag!("grpc_port", 3232 as u16, "The port to bind to for grpc.");
    let web_port = define_flag!(
        "web_port",
        3233 as u16,
        "The port to bind to for the webserver."
    );
    let data_dir = define_flag!("data_dir", String::new(), "The data directory to use");
    let secret_key = define_flag!("secret_key", String::new(), "The auth secret key");

    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth"),
        "The auth service hostname"
    );
    let auth_port = define_flag!("auth_port", 8888, "The auth service port");
    let disable_auth = define_flag!("disable_auth", false, "Whether to disable auth");
    let base_url = define_flag!(
        "base_url",
        String::from("http://localhost:3233"),
        "The base url of the service"
    );
    parse_flags!(
        grpc_port,
        web_port,
        data_dir,
        secret_key,
        auth_hostname,
        auth_port,
        disable_auth,
        base_url
    );

    let auth = if disable_auth.value() {
        auth_client::AuthClient::new_fake()
    } else {
        auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value())
    };
    auth.global_init(secret_key.value());

    let handler = server_lib::LoggerServiceHandler::new(data_dir.value());
    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());
    server.add_service(logger_grpc_rust::LoggerServiceServer::new_service_def(
        handler.clone(),
    ));

    let handler2 = handler.clone();
    std::thread::spawn(move || {
        handler2.reorganize();
        std::thread::sleep(std::time::Duration::from_secs(300));
    });

    let _server = server.build().expect("server");

    ws::serve(
        webserver::LoggerWebServer::new(handler, auth, base_url.value()),
        web_port.value(),
    )
    .await;

    loop {
        std::thread::park();
    }
}

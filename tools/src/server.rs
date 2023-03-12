use std::sync::Arc;

#[tokio::main]
async fn main() {
    let data_dir = flags::define_flag!(
        "data_directory",
        String::new(),
        "The directory where data is stored and loaded from"
    );

    let hostname = flags::define_flag!(
        "hostname",
        String::from("localhost:4959"),
        "The hostname of the src server"
    );

    let auth_hostname = flags::define_flag!(
        "auth_hostname",
        String::new(),
        "The hostname of the auth service (if empty, no authentication required)"
    );

    let auth_port = flags::define_flag!("auth_port", 8888, "The port of the auth service");

    flags::parse_flags!(data_dir, hostname, auth_hostname, auth_port);

    if data_dir.value().is_empty() {
        eprintln!("ERROR: A data directory must be specified! (--data_directory)");
        std::process::exit(1);
    }

    std::fs::create_dir_all(data_dir.value()).ok();

    let auth_host = auth_hostname.value();
    let auth: Arc<dyn server_service::auth::AuthPlugin> = if auth_host.is_empty() {
        Arc::new(server_service::auth::FakeAuthPlugin::new())
    } else {
        let client = auth_client::AuthClient::new(&auth_host, auth_port.value());
        Arc::new(server_service::auth::AuthServicePlugin::new(
            client,
            auth_host,
            auth_port.value(),
        ))
    };

    let data_path = std::path::PathBuf::from(data_dir.value());
    let table = server_service::SrcServer::new(data_path, hostname.value(), auth)
        .expect("failed to create src server");

    let handler = Arc::new(table);
    let _h = handler.clone();
    std::thread::spawn(move || {
        _h.monitor_memory();
    });

    let s = service::SrcServerService(handler);
    bus_rpc::serve(4959, s).await;
}

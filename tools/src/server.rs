use std::sync::Arc;

#[tokio::main]
async fn main() {
    let data_dir = flags::define_flag!(
        "data_directory",
        String::new(),
        "The directory where data is stored and loaded from"
    );

    flags::parse_flags!(data_dir);

    if data_dir.value().is_empty() {
        eprintln!("ERROR: A data directory must be specified! (--data_directory)");
        std::process::exit(1);
    }

    std::fs::create_dir_all(data_dir.value()).ok();

    let data_path = std::path::PathBuf::from(data_dir.value());
    let table = server_service::SrcServer::new(data_path).expect("failed to create src server");

    let handler = Arc::new(table);
    let _h = handler.clone();
    std::thread::spawn(move || {
        _h.monitor_memory();
    });

    let s = service::SrcServerService(handler);
    bus_rpc::serve(4959, s).await;
}

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

    let data_path = std::path::PathBuf::from(data_dir.value());
    let d = daemon_service::SrcDaemon::new(data_path).expect("failed to create daemon server");
    let handler = Arc::new(d);
    let _h = handler.clone();

    let s = service::SrcDaemonService(handler);
    bus_rpc::serve(5969, s).await;
}

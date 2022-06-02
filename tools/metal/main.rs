use state::MetalStateManager;

use std::sync::Arc;

fn main() {
    let root_dir = std::path::PathBuf::from("/tmp/metal");
    let ip_address = "127.0.0.1".parse().expect("failed to parse IP address");

    //let state_mgr = state::FilesystemState::new(root_dir.clone());
    let state_mgr = state::FakeState::new();
    state_mgr.initialize().unwrap();

    let monitor = Arc::new(monitor::MetalMonitor::new(root_dir.clone(), ip_address));

    let handler = service::MetalServiceHandler::new(Arc::new(state_mgr), monitor.clone())
        .expect("failed to create service handler");

    monitor.set_coordinator(handler.0.clone());

    // Start monitoring thread
    let _mon = monitor.clone();
    std::thread::spawn(move || {
        _mon.monitor();
    });

    // Start restart_loop thread
    std::thread::spawn(move || {
        monitor.restart_loop();
    });

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(20202);
    server.add_service(metal_grpc_rust::MetalServiceServer::new_service_def(
        handler,
    ));

    let _server = server.build().expect("failed to build server");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

extern crate fuse;
extern crate grpc;
extern crate libc;
extern crate pool;
extern crate time;

#[macro_use]
extern crate flags;
extern crate largetable_client;
extern crate largetable_test;
extern crate protobuf;
extern crate tls_api_native_tls;
extern crate weld;
extern crate weld_repo;

mod client_service;
mod fs;
mod parallel_fs;

use std::sync::Arc;

fn main() {
    let mount_point = define_flag!(
        "mount_point",
        String::from("/mnt"),
        "The path to mount the virtual filesystem to"
    );
    let mount = define_flag!(
        "mount",
        true,
        "Whether or not to try to mount the FUSE filesystem"
    );
    let port = define_flag!("port", 8008, "The port to bind to.");
    let weld_hostname = define_flag!(
        "weld_hostname",
        String::from("localhost"),
        "the hostname for the remote weld service"
    );
    let server_port = define_flag!("server_port", 8001, "the port to connect to");
    parse_flags!(mount_point, mount, weld_hostname, port, server_port);

    let db = largetable_test::LargeTableMockClient::new();
    let batching_client = Arc::new(batching_client::LargeTableBatchingClient::new_with_cache(
        db,
    ));
    let mut repo = weld_repo::Repo::new_with_client(batching_client.clone());

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let count = batching_client.flush();
        if count > 0 {
            println!("flushed {}", count);
        }
    });

    let client =
        weld::WeldServerClient::new(&weld_hostname.value(), String::new(), server_port.value());
    repo.add_remote_server(client);

    // Start gRPC service.
    let mut handler = client_service::WeldLocalServiceHandler::new(repo.clone());
    handler.set_mount_dir(mount_point.value());

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(weld::WeldLocalServiceServer::new_service_def(
        handler.clone(),
    ));
    server.http.set_cpu_pool_threads(16);

    let _server = server.build().expect("server");

    // Mount filesystem.
    let filesystem = parallel_fs::WeldParallelFs::new(repo);

    if mount.value() {
        let options = [
            "-o",
            "fsname=hello",
            "async_read=true",
            "negative_timeout=5",
            "debug=true",
        ]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&std::ffi::OsStr>>();
        ::fuse::mount(filesystem, &mount_point.value(), &options).unwrap();
    }

    std::thread::park();
}

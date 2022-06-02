extern crate fuse;
extern crate grpc;
extern crate libc;
extern crate pool;
extern crate rand;
extern crate time;

#[macro_use]
extern crate flags;
extern crate client_service;
extern crate largetable_client;
extern crate largetable_test;
extern crate protobuf;
extern crate tls_api_stub;
extern crate weld;
extern crate weld_repo;

mod fs;
mod parallel_fs;

use std::fs::File;
use std::io::Read;
use std::sync::Arc;

fn main() {
    let mount_point = define_flag!(
        "mount_point",
        String::from("/tmp/code"),
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
    let username = define_flag!("username", String::from(""), "The username to use.");
    let largetable_hostname = define_flag!(
        "largetable_hostname",
        String::from("127.0.0.1"),
        "the hostname of the largetable service"
    );
    let largetable_port = define_flag!("largetable_port", 50051, "the on the largetable service");
    let use_tls = define_flag!("use_tls", true, "Whether or not to use TLS encryption");
    let tls_hostname = define_flag!(
        "tls_hostname",
        String::from("server.weld.io"),
        "the hostname to require the server to authenticate itself as"
    );
    let root_ca = define_flag!(
        "root_ca",
        String::from(""),
        "path to a file containing the root CA .der file"
    );
    let cert = define_flag!(
        "cert",
        String::from(""),
        "path to a file containing the client cert .der file"
    );
    parse_flags!(
        mount_point,
        mount,
        weld_hostname,
        port,
        server_port,
        username,
        largetable_hostname,
        largetable_port,
        use_tls,
        tls_hostname,
        root_ca,
        cert
    );

    let db = largetable_client::LargeTableRemoteClient::new(
        &largetable_hostname.value(),
        largetable_port.value(),
    );
    let batching_client = Arc::new(batching_client::LargeTableBatchingClient::new_with_cache(
        db,
    ));
    let mut repo = weld_repo::Repo::new_with_client(batching_client.clone());

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let count = batching_client.flush();
    });

    if use_tls.value() {
        let client = weld::WeldServerClient::new_tls(&weld_hostname.value(), server_port.value());
        repo.add_remote_server(client);
    } else {
        let client = weld::WeldServerClient::new(
            &weld_hostname.value(),
            username.value(),
            server_port.value(),
        );
        repo.add_remote_server(client);
    }

    // Start gRPC service.
    let mut handler = client_service::WeldLocalServiceHandler::new(repo.clone());
    handler.set_mount_dir(mount_point.path());

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(weld::WeldLocalServiceServer::new_service_def(
        handler.clone(),
    ));

    let _server = server.build().expect("server");

    // Mount filesystem.
    let filesystem = parallel_fs::WeldParallelFs::new(repo);

    if mount.value() {
        let options = [
            "-o",
            "fsname=hello",
            "async_read=true",
            "negative_timeout=5",
        ]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&std::ffi::OsStr>>();
        ::fuse::mount(filesystem, &mount_point.path(), &options).unwrap();
    }

    std::thread::park();
}

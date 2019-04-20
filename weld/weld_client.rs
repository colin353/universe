extern crate fuse;
extern crate grpc;
extern crate libc;
extern crate time;

#[macro_use]
extern crate flags;
extern crate largetable_client;
extern crate largetable_test;
extern crate tls_api_native_tls;
extern crate weld;
extern crate weld_repo;

mod client_service;
mod fs;

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
        String::from("localhost:8001"),
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
        root_ca
    );

    let db = largetable_client::LargeTableRemoteClient::new(
        &largetable_hostname.value(),
        largetable_port.value(),
    );
    let mut repo = weld_repo::Repo::new(Arc::new(db));

    if use_tls.value() {
        let mut cert_contents = Vec::new();
        File::open(root_ca.value())
            .unwrap()
            .read_to_end(&mut cert_contents)
            .unwrap();
        let client = weld::WeldServerClient::new_tls(
            &weld_hostname.value(),
            &tls_hostname.value(),
            username.value(),
            server_port.value(),
            cert_contents,
        );
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
    let handler = client_service::WeldLocalServiceHandler::new(repo.clone());

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(weld::WeldLocalServiceServer::new_service_def(
        handler.clone(),
    ));
    server.http.set_cpu_pool_threads(4);

    let _server = server.build().expect("server");

    // Mount filesystem.
    let filesystem = fs::WeldFS::new(repo);

    if mount.value() {
        let options = ["-o", "fsname=hello"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&std::ffi::OsStr>>();
        ::fuse::mount(filesystem, &mount_point.value(), &options).unwrap();
    } else {
        std::thread::park();
    }
}

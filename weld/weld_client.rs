extern crate fuse;
extern crate grpc;
extern crate libc;
extern crate time;

#[macro_use]
extern crate flags;
extern crate largetable_client;
extern crate largetable_test;
extern crate weld;
extern crate weld_repo;

mod client_service;
mod fs;

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
    let cert = define_flag!(
        "cert",
        String::from(""),
        "Where to look up the TLS certificate."
    );
    let key = define_flag!(
        "key",
        String::from(""),
        "Where to look up the TLS private key."
    );
    let root_cert = define_flag!(
        "root_cert",
        String::from(""),
        "Where to look up the root CA public key."
    );
    let username = define_flag!("username", String::from(""), "The username to use.");
    parse_flags!(
        mount_point,
        mount,
        weld_hostname,
        cert,
        key,
        root_cert,
        username
    );

    let db = largetable_test::LargeTableMockClient::new();
    let mut repo = weld_repo::Repo::new(db);

    let certificate = std::fs::read(cert.value()).unwrap();
    let private_key = std::fs::read(key.value()).unwrap();
    let root_cert = std::fs::read(root_cert.value()).unwrap();
    let client =
        weld::WeldServerClient::new(&weld_hostname.value(), username.value(), port.value());
    repo.add_remote_server(client);

    // Start gRPC service.
    let handler = client_service::WeldLocalServiceHandler::new(repo.clone());

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
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

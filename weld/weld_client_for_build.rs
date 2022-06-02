extern crate fuse;
extern crate grpc;
extern crate libc;
extern crate pool;
extern crate rand;
extern crate time;

#[macro_use]
extern crate flags;
extern crate build_consumer;
extern crate chat_client;
extern crate client_service;
extern crate largetable_client;
extern crate largetable_test;
extern crate lockserv_client;
extern crate protobuf;
extern crate queue_client;
extern crate tls_api_openssl;
extern crate weld;
extern crate weld_repo;

mod fs;
mod parallel_fs;

use queue_client::Consumer;
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
    let chat_hostname = define_flag!(
        "chat_hostname",
        String::from("chat"),
        "The server for the chat client"
    );
    let chat_port = define_flag!("chat_port", 16668, "The grpc port of the chat service");
    let weld_hostname = define_flag!(
        "weld_hostname",
        String::from("localhost"),
        "the hostname for the remote weld service"
    );
    let server_port = define_flag!("server_port", 8001, "the port to connect to");
    let lockserv_hostname = define_flag!(
        "lockserv_hostname",
        String::from("lockserv"),
        "the hostname of the lock service"
    );
    let lockserv_port = define_flag!("lockserv_port", 5555, "the hostname of the lock service");
    let queue_hostname = define_flag!(
        "queue_hostname",
        String::from("queue"),
        "the hostname of the queue service"
    );
    let queue_port = define_flag!("queue_port", 5554, "the port of the queue service");
    let auth_token = define_flag!(
        "auth_token",
        String::new(),
        "the auth token to use for weld client"
    );
    parse_flags!(
        mount_point,
        mount,
        weld_hostname,
        port,
        server_port,
        lockserv_hostname,
        lockserv_port,
        queue_hostname,
        queue_port,
        chat_hostname,
        chat_port,
        auth_token
    );

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
    client.set_permanent_token(auth_token.value());
    repo.add_remote_server(client.clone());

    // Start gRPC service.
    let mut handler = client_service::WeldLocalServiceHandler::new(repo.clone());
    handler.set_mount_dir(mount_point.value());

    let mut server = grpc::ServerBuilder::<tls_api_openssl::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(weld::WeldLocalServiceServer::new_service_def(
        handler.clone(),
    ));

    let _server = server.build().expect("server");

    // Mount filesystem.
    let filesystem = parallel_fs::WeldParallelFs::new(repo);

    // Connect to the chat service
    let chat_client =
        chat_client::ChatClient::new(&chat_hostname.value(), chat_port.value(), false);

    let lockserv_client =
        lockserv_client::LockservClient::new(&lockserv_hostname.value(), lockserv_port.value());
    let queue_client = queue_client::QueueClient::new(&queue_hostname.value(), queue_port.value());
    let build_consumer = build_consumer::BuildConsumer::new(
        handler.clone(),
        queue_client.clone(),
        lockserv_client.clone(),
        chat_client,
    );
    let presubmit_consumer =
        build_consumer::PresubmitConsumer::new(queue_client.clone(), lockserv_client.clone());
    let submit_consumer =
        build_consumer::SubmitConsumer::new(handler, client, queue_client, lockserv_client);

    std::thread::spawn(move || {
        build_consumer.start(String::from("builds"));
    });
    std::thread::spawn(move || {
        presubmit_consumer.start(String::from("presubmit"));
    });
    std::thread::spawn(move || {
        submit_consumer.start(String::from("submit"));
    });

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

    loop {
        std::thread::park();
    }
}

extern crate futures;
extern crate glob;
extern crate grpc;
extern crate protobuf;
extern crate time;
#[macro_use]
extern crate flags;
extern crate largetable;
extern crate tls_api;
extern crate tls_api_stub;

extern crate compaction;
extern crate largetable_grpc_rust;
extern crate largetable_proto_rust;
extern crate sstable;
mod server_service;

#[cfg(test)]
extern crate test;

use std::thread;

fn main() {
    let port = define_flag!("port", 50051 as u16, "The port to bind to.");
    let memory_limit = define_flag!(
        "memory_limit",
        100_000_000,
        "The limit at which to dump mtables to disk (in bytes)."
    );
    let data_directory = define_flag!(
        "data_directory",
        String::from("./data"),
        "The directory where data is stored and loaded from."
    );
    parse_flags!(port, data_directory, memory_limit);

    let mut handler =
        server_service::LargeTableServiceHandler::new(memory_limit.value(), data_directory.value());

    // Read any existing dtables from disk.
    handler.load_existing_dtables();

    // Read any journals.
    handler.load_existing_journals();

    // Create a new journal for this session.
    handler.add_journal();

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(
        largetable_grpc_rust::LargeTableServiceServer::new_service_def(handler.clone()),
    );
    server.http.set_cpu_pool_threads(32);

    let _server = server.build().expect("server");

    loop {
        thread::sleep(std::time::Duration::from_secs(60));
        handler.check_memory();
        handler.check_compaction();
    }
}

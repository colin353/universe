extern crate futures;
extern crate glob;
extern crate grpc;
extern crate protobuf;
extern crate time;
#[macro_use]
extern crate flags;
extern crate compaction;
extern crate largetable;
extern crate largetable_grpc_rust;
extern crate largetable_proto_rust;
extern crate logger_client;
extern crate sstable;
extern crate tls_api;
extern crate tls_api_stub;
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
    let logger_hostname = define_flag!(
        "logger_hostname",
        String::from(""),
        "The hostname of the logger service. If empty, just log to stdout"
    );
    let logger_port = define_flag!("logger_port", 3232, "Port of the logger service");
    parse_flags!(
        port,
        data_directory,
        memory_limit,
        logger_hostname,
        logger_port
    );

    let logger = if logger_hostname.value().is_empty() {
        logger_client::LoggerClient::new_stdout()
    } else {
        logger_client::LoggerClient::new(&logger_hostname.value(), logger_port.value())
    };

    let mut handler = server_service::LargeTableServiceHandler::new(
        memory_limit.value(),
        data_directory.path(),
        logger.clone(),
    );

    std::thread::spawn(move || {
        logger.start_logging();
    });

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.add_service(
        largetable_grpc_rust::LargeTableServiceServer::new_service_def(handler.clone()),
    );
    server.http.set_cpu_pool_threads(32);

    let _server = server.build().expect("server");

    // Read any existing dtables from disk.
    handler.load_existing_dtables();

    // Read any journals.
    handler.load_existing_journals();

    // Create a new journal for this session.
    handler.add_journal();

    // Indicate that requests can now be processed
    handler.ready();

    loop {
        thread::sleep(std::time::Duration::from_secs(60));
        handler.check_memory();
        handler.check_compaction();
    }
}

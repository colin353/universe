#[macro_use]
extern crate flags;

fn main() {
    let port = define_flag!("port", 9898, "The port to bind to");
    let index_dir = define_flag!(
        "index_dir",
        String::new(),
        "The directory of the search index."
    );

    parse_flags!(port, index_dir);

    let mut server = grpc::ServerBuilder::<tls_api_stub::TlsAcceptor>::new();
    server.http.set_port(port.value());
    server.http.set_cpu_pool_threads(2);

    let searcher = search_lib::Searcher::new(&index_dir.path());
    let handler = server_lib::SearchServiceHandler::new(searcher);
    server.add_service(search_grpc_rust::SearchServiceServer::new_service_def(
        handler,
    ));
    let _server = server.build().unwrap();

    loop {
        std::thread::park();
    }
}

extern crate grpc;

#[macro_use]
extern crate flags;
extern crate largetable_test;
extern crate server_impl;
extern crate task_webserver;
extern crate ws;

use ws::Server;

fn main() {
    let grpc_port = define_flag!(
        "grpc_port",
        7777,
        "The port to bind to for the grpc service"
    );
    let web_port = define_flag!("web_port", 7878, "The port to bind to for the web service");
    parse_flags!(grpc_port, web_port);

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());
    server.http.set_cpu_pool_threads(8);

    let database = largetable_test::LargeTableMockClient::new();
    let handler = server_impl::TaskServiceHandler::new(database.clone());
    server.add_service(tasks_grpc_rust::TaskServiceServer::new_service_def(handler));

    let _server = server.build().expect("server");
    task_webserver::TaskWebServer::new(database).serve(web_port.value());
}

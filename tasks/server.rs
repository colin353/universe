extern crate grpc;

#[macro_use]
extern crate flags;
extern crate auth_client;
extern crate largetable_test;
extern crate server_impl;
extern crate task_lib;
extern crate task_webserver;
extern crate ws;

use task_lib::TaskServerConfiguration;
use ws::Server;

fn main() {
    let grpc_port = define_flag!(
        "grpc_port",
        7777,
        "The port to bind to for the grpc service"
    );
    let web_port = define_flag!("web_port", 7878, "The port to bind to for the web service");
    let weld_client_hostname = define_flag!(
        "weld_client_hostname",
        String::from("localhost"),
        "The weld local service hostname"
    );
    let weld_client_port = define_flag!("weld_client_port", 8001, "The weld service hostname");

    let weld_server_hostname = define_flag!(
        "weld_server_hostname",
        String::from("localhost"),
        "The weld server service hostname"
    );
    let weld_server_port = define_flag!("weld_server_port", 8001, "The weld server service port");
    let base_url = define_flag!(
        "base_url",
        String::from("http://tasks.colinmerkel.xyz"),
        "the base URL of the tasks webservice"
    );
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::new(),
        "the hostname of the auth service"
    );
    let auth_port = define_flag!("auth_port", 8888, "the port of the auth service");
    let x20_hostname = define_flag!(
        "x20_hostname",
        String::new(),
        "the host for the x20 service"
    );
    let x20_port = define_flag!("x20_port", 8001, "the port of the x20 service");
    let secret_key = define_flag!(
        "secret_key",
        String::new(),
        "the service account secret key"
    );
    parse_flags!(
        grpc_port,
        web_port,
        weld_client_hostname,
        weld_client_port,
        weld_server_hostname,
        weld_server_port,
        base_url,
        auth_hostname,
        auth_port,
        x20_hostname,
        x20_port,
        secret_key
    );

    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());
    server.http.set_cpu_pool_threads(8);

    let mut config = TaskServerConfiguration::new();
    config.weld_client_hostname = weld_client_hostname.value();
    config.weld_client_port = weld_client_port.value();
    config.base_url = base_url.value();
    config.weld_server_hostname = weld_server_hostname.value();
    config.weld_server_port = weld_server_port.value();
    config.x20_hostname = x20_hostname.value();
    config.x20_port = x20_port.value();
    config.secret_key = secret_key.value();

    let database = largetable_test::LargeTableMockClient::new();
    let handler = server_impl::TaskServiceHandler::new(config, database.clone());
    server.add_service(tasks_grpc_rust::TaskServiceServer::new_service_def(handler));

    let _server = server.build().expect("server");
    task_webserver::TaskWebServer::new(database, auth, base_url.value()).serve(web_port.value());
}

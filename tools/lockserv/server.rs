#[macro_use]
extern crate flags;

#[tokio::main]
async fn main() {
    let port = define_flag!("port", 5555, "The port to bind to");
    parse_flags!(port);

    let h = server_lib::LockServiceHandler::new();
    let handler = lockserv_bus::LockAsyncService(std::sync::Arc::new(h));

    bus_rpc::serve(port.value(), handler).await;
}

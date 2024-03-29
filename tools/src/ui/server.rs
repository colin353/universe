#[tokio::main]
async fn main() {
    let src_metal = flags::define_flag!(
        "src_metal",
        String::new(),
        "the metal service name for the src service"
    );
    let base_url = flags::define_flag!(
        "base_url",
        String::from("https://src.colinmerkel.xyz"),
        "the base URL of the server"
    );
    let auth_metal = flags::define_flag!(
        "auth_metal",
        String::new(),
        "the metal service name for the auth service"
    );
    let queue_metal = flags::define_flag!(
        "queue_metal",
        String::new(),
        "the metal service name for the queue service"
    );
    let port = flags::define_flag!("port", 8080_u16, "the port to bind to");

    flags::parse_flags!(src_metal, port, auth_metal);

    let auth_metal = auth_metal.value();
    let auth = if !auth_metal.is_empty() {
        Some(auth_client::AuthAsyncClient::new_metal(&auth_metal))
    } else {
        None
    };

    let queue_metal = queue_metal.value();
    let queue = if !queue_metal.is_empty() {
        let connector = std::sync::Arc::new(bus_rpc::MetalAsyncClient::new(queue_metal));
        Some(queue_bus::QueueAsyncClient::new(connector))
    } else {
        None
    };

    let src_metal = src_metal.value();
    let server = if !src_metal.is_empty() {
        server_lib::SrcUIServer::new_metal(src_metal, base_url.value(), auth, queue)
    } else {
        server_lib::SrcUIServer::new("127.0.0.1".to_string(), 4959, base_url.value(), auth, queue)
    };

    ws::serve(server, port.value()).await;
}

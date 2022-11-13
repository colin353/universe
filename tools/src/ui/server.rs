use ws::Server;

#[tokio::main]
async fn main() {
    ws::serve(server_lib::SrcUIServer::new("127.0.0.1".to_string(), 4959), 8080).await;
}

use futures::stream::StreamExt;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let id = flags::define_flag!("id", 0, "the fortune's ID");
    flags::parse_flags!(id);

    let connector = Arc::new(bus_rpc::HyperClient::new(String::from("127.0.0.1"), 4521));
    let client = fortune_bus::FortuneAsyncClient::new(connector);
    let resp = client
        .fortune(fortune_bus::FortuneRequest {
            fortune_id: id.value() as u32,
        })
        .await
        .unwrap();
    println!("{}", resp.fortune);

    // Now stream them
    println!("streaming...");

    let mut resp = client
        .fortune_stream(fortune_bus::FortuneRequest {
            fortune_id: id.value() as u32,
        })
        .await
        .unwrap();

    while let Some(item) = resp.next().await {
        println!("{}", item.fortune);
    }
}

use std::sync::Arc;

fn main() {
    let id = flags::define_flag!("id", 0, "the fortune's ID");
    flags::parse_flags!(id);

    let connector = Arc::new(bus_rpc::HyperSyncClient::new(
        String::from("127.0.0.1"),
        4521,
    ));
    let client = fortune_bus::FortuneClient::new(connector);
    let resp = client
        .fortune(fortune_bus::FortuneRequest {
            fortune_id: id.value() as u32,
        })
        .unwrap();
    println!("{}", resp.fortune);
}

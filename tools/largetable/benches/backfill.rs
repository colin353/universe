use rand::Rng;
use std::io::Read;
use std::sync::Arc;

fn write(client: service::LargeTableClient) {
    let mut rng = rand::thread_rng();
    loop {
        let key_bytes = rng.gen::<[u8; 16]>();
        let mut key = String::new();
        for byte in key_bytes {
            key.push_str(&format!("{:x?}", byte));
        }

        let mut data_bytes = [0_u8; 1024];
        rng.fill(&mut data_bytes);
        let result = client
            .write(service::WriteRequest {
                row: String::from("values"),
                column: key,
                timestamp: 0,
                data: data_bytes.to_vec(),
            })
            .expect("failed to write to largetable");
    }
}

fn main() {
    let host = flags::define_flag!(
        "host",
        String::from("127.0.0.1"),
        "the hostname of the largetable service"
    );
    let port = flags::define_flag!("port", 4321, "the port of the largetable service");
    let timestamp = flags::define_flag!("timestamp", 0, "the timestamp to use when querying");
    let args = flags::parse_flags!(host, port);

    let connector = Arc::new(bus_rpc::HyperClient::new(host.value(), port.value()));
    let client = service::LargeTableClient::new(connector);

    for _ in 0..32 {
        let _c = client.clone();
        std::thread::spawn(move || {
            write(_c);
        });
    }
    loop {
        std::thread::park();
    }
}

use rand::Rng;
use std::io::Read;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

fn read(client: service::LargeTableClient, query_count: Arc<AtomicU32>) {
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
            .read_range(service::ReadRangeRequest {
                row: String::from("values"),
                filter: service::Filter {
                    min: key,
                    ..Default::default()
                },
                timestamp: 0,
                limit: 16,
            })
            .expect("failed to read from largetable");

        query_count.fetch_add(1, Ordering::SeqCst);
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

    let query_count = Arc::new(AtomicU32::new(0));

    for _ in 0..8 {
        let _c = client.clone();
        let q = query_count.clone();
        std::thread::spawn(move || {
            read(_c, q);
        });
    }

    loop {
        let t = std::time::Instant::now();
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!(
            "{} QPS",
            query_count.swap(0, Ordering::SeqCst) as f64 / t.elapsed().as_secs_f64()
        );
    }
}

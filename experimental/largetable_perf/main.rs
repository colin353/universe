use largetable_client::LargeTableClient;
use rand::{thread_rng, Rng};

#[macro_use]
extern crate flags;

pub fn deserialize_key(key: &str) -> (&str, &str) {
    let split: Vec<&str> = key.split("\x00").collect();
    (split[0], split[1])
}

fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = core::ptr::read_volatile(&dummy);
        core::mem::forget(dummy);
        ret
    }
}

fn main() {
    let file = define_flag!("file", String::from(""), "The filename to read");
    parse_flags!(file);
    let f = std::fs::File::open(file.value()).unwrap();
    let mut r = sstable2::SSTableReader::<largetable_proto_rust::Record>::new(f).unwrap();
    let mut indices = Vec::new();
    for (key, _) in r {
        indices.push(key);
    }

    let f = std::fs::File::open(file.value()).unwrap();
    let mut r = sstable2::SSTableReader::<largetable_proto_rust::Record>::new(f).unwrap();
    thread_rng().shuffle(&mut indices[0..10000]);

    let indices2 = std::sync::Arc::new(indices);
    let client = std::sync::Arc::new(largetable_client::LargeTableRemoteClient::new(
        "localhost",
        2020,
    ));
    let pool = pool::ThreadPool::new(32);

    loop {
        let t = std::time::Instant::now();
        for index in 0..10000 {
            let idxs = indices2.clone();
            let client = client.clone();
            pool.execute(move || {
                let (row, col) = deserialize_key(&idxs[index % idxs.len()]);
                let start = std::time::Instant::now();
                let x = client.read(row, col, 0);
                black_box(x);
            })
        }
        pool.join();

        println!("10k reads in {} us", t.elapsed().as_micros());
        println!("{} QPS", 10000.0 * 1e6 / t.elapsed().as_micros() as f64);
    }
}

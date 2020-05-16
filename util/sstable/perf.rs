use rand::{thread_rng, Rng};

fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = std::ptr::read_volatile(&dummy);
        std::mem::forget(dummy);
        ret
    }
}

fn main() {
    let start = std::time::Instant::now();
    let f = std::fs::File::open("/home/colin/Documents/code/index/code.sstable").unwrap();
    let buf = std::io::BufReader::new(f);
    let mut reader = sstable::SSTableReader::<search_proto_rust::File>::new(Box::new(buf)).unwrap();

    println!("Loaded SSTable in {} us", start.elapsed().as_micros());
    let start = std::time::Instant::now();

    // Fill a vector with keys from the SSTable
    let mut keys = Vec::new();
    for (k, v) in &mut reader {
        keys.push(k);
    }
    println!("Read entire SSTable in {} us", start.elapsed().as_micros());

    loop {
        let start = std::time::Instant::now();
        let mut reads = 0;
        loop {
            thread_rng().shuffle(&mut keys);
            for key in &keys {
                black_box(reader.get(key).unwrap());
                reads += 1;
                if reads > 10000 {
                    break;
                }
            }
            if reads > 10000 {
                break;
            }
        }
        println!("10k reads in {} us", start.elapsed().as_micros());
        println!("{} QPS", 10000.0 * 1e6 / start.elapsed().as_micros() as f64);
    }
}

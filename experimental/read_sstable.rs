extern crate rand;
extern crate sstable;
#[macro_use]
extern crate flags;
extern crate largetable_proto_rust;

use rand::{thread_rng, Rng};
use std::fs::File;
use std::io::BufReader;

fn main() {
    let file = define_flag!("file", String::from(""), "The filename to read");
    parse_flags!(file);

    let f = Box::new(BufReader::with_capacity(
        64000,
        File::open(file.value()).unwrap(),
    ));
    let mut r = sstable::SSTableReader::<largetable_proto_rust::Record>::new(f).unwrap();

    let mut indices = Vec::new();
    for (key, _) in r {
        indices.push(key);
    }

    let f = Box::new(BufReader::with_capacity(
        64000,
        File::open(file.value()).unwrap(),
    ));
    let mut r = sstable::SSTableReader::<largetable_proto_rust::Record>::new(f).unwrap();

    thread_rng().shuffle(&mut indices[0..10000]);

    loop {
        let t = std::time::Instant::now();
        for index in 0..10000 {
            r.get(&indices[index % indices.len()]).unwrap();
        }
        println!("10k reads in {} us", t.elapsed().as_micros());
        println!("{} QPS", 10000.0 * 1e6 / t.elapsed().as_micros() as f64);
        std::io::stdin().read_line(&mut String::new()).unwrap();
    }
}

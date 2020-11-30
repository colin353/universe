extern crate sstable;
extern crate sstable2;
#[macro_use]
extern crate flags;

use std::fs::File;

fn main() {
    let input = define_flag!("input", String::from(""), "The filename to read");
    let output = define_flag!("output", String::from(""), "The filename to write");
    parse_flags!(input, output);

    let f = Box::new(File::open(input.path()).unwrap());
    let mut r = sstable::SSTableReader::<primitive::Primitive<Vec<u8>>>::new(f).unwrap();

    let f2 = File::create(output.path()).unwrap();
    let mut w = sstable2::SSTableBuilder::new(f2);

    for (key, val) in r {
        w.write_ordered(&key, val);
    }

    w.finish();
}

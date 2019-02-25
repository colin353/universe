extern crate largetable_proto_rust;
extern crate recordio;

use std::fs;

use largetable_proto_rust::Record;

fn main() {
    let f = fs::File::open("/usr/local/largetable/data-0005.recordio").unwrap();
    let records = recordio::RecordIOReaderOwned::<Record>::new(Box::new(f));
    for record in records {
        println!(
            "record: {}::{} ({} bytes)",
            record.get_row(),
            record.get_col(),
            record.get_data().len()
        );
    }
}

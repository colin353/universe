fn main() {
    let client = largetable_client::LargeTableRemoteClient::new("localhost", 2020);

    let start = std::time::Instant::now();

    let iter: Vec<_> = largetable_client::LargeTableScopedIterator::<weld::Change, _>::new(
        &client,
        String::from("metadata"),
        String::new(),
        String::new(),
        String::new(),
        0,
    )
    .collect();

    println!("length: {}", iter.len());
    println!("time: {}", start.elapsed().as_micros());
}

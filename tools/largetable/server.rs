mod largetable_service;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    let data_dir = flags::define_flag!(
        "data_directory",
        String::new(),
        "The directory where data is stored and loaded from"
    );

    flags::parse_flags!(data_dir);

    if data_dir.value().is_empty() {
        eprintln!("ERROR: A data directory must be specified! (--data_directory)");
        std::process::exit(1);
    }

    let mut journals = Vec::new();
    let mut table = largetable::LargeTable::new();
    table.add_mtable();

    let data_path = std::path::PathBuf::from(data_dir.value());
    let dtable_extension = std::ffi::OsStr::new("dtable");
    let journal_extension = std::ffi::OsStr::new("journal");
    for path in std::fs::read_dir(&data_path).expect("failed to read from data_directory!") {
        let path = path
            .expect("failed to access file in data directory")
            .path();
        if let Some(ext) = path.extension() {
            if ext == dtable_extension {
                let f = std::fs::File::open(&path).expect("failed to open dtable!");
                let dt = largetable::DTable::from_file(f).expect("failed to load dtable");
                table.add_dtable(dt);
                println!("added dtable: {:?}", path);
            } else if ext == journal_extension {
                let f = std::fs::File::open(&path).expect("failed to open journal!");
                let r = std::io::BufReader::new(f);
                table
                    .load_from_journal(r)
                    .expect("failed to load from journal!");
                println!("added journal: {:?}", path);
                journals.push(path);
            }
        }
    }

    // Create a fresh journal to use for this startup
    let journal_path = data_path.join(format!("{}.journal", largetable_service::timestamp_usec()));
    let f = std::fs::File::create(&journal_path).expect("failed to create journal!");
    table.add_journal(std::io::BufWriter::new(f));
    journals.insert(0, journal_path);

    let handler = Arc::new(largetable_service::LargeTableHandler::new(table, journals));
    let _h = handler.clone();
    std::thread::spawn(move || {
        largetable_service::monitor_memory(data_path, _h);
    });

    let s = service::LargeTableService(handler);
    bus_rpc::serve(4321, s).await;
}

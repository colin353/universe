use std::sync::Arc;

fn main() {
    let host = flags::define_flag!(
        "host",
        String::from("127.0.0.1"),
        "the hostname of the largetable service"
    );
    let port = flags::define_flag!("port", 4321, "the port of the largetable service");
    let timestamp = flags::define_flag!("timestamp", 0, "the timestamp to use when querying");
    let args = flags::parse_flags!(host, port);

    let strargs: Vec<_> = args.iter().map(|s| s.as_str()).collect();

    let connector = Arc::new(bus_rpc::HyperSyncClient::new(host.value(), port.value()));
    let client = service::LargeTableClient::new(connector);

    match strargs.get(0) {
        Some(&"read") => match (strargs.get(1), strargs.get(2), strargs.get(3)) {
            (Some(ref row), Some(ref col), None) => {
                let result = client
                    .read(service::ReadRequest {
                        row: row.to_string(),
                        column: row.to_string(),
                        timestamp: timestamp.value(),
                    })
                    .expect("failed to read from largetable");
                if result.found {
                    println!("{:x?}", result.data);
                    if let Ok(s) = std::str::from_utf8(&result.data) {
                        println!("{:?}", s);
                    }
                } else {
                    println!("<no data>");
                }
            }
            _ => {
                eprintln!("you must specify a row and column to read from");
            }
        },
        Some(&"write") => match (strargs.get(1), strargs.get(2), strargs.get(3)) {
            (Some(ref row), Some(ref col), Some(data)) => {
                let result = client
                    .write(service::WriteRequest {
                        row: row.to_string(),
                        column: row.to_string(),
                        timestamp: timestamp.value(),
                        data: data.as_bytes().to_owned(),
                    })
                    .expect("failed to write to largetable");
            }
            _ => {
                eprintln!("you must specify a row, column, and data to write to");
            }
        },
        _ => {
            println!("specify an argument: read or write");
        }
    }
}

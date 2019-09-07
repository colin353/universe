use std::io::{self, Write};

#[macro_use]
extern crate flags;
extern crate largetable_client;
use largetable_client::LargeTableClient;

fn main() {
    let host = define_flag!(
        "host",
        String::from("localhost"),
        "The hostname:port of the largetable service"
    );
    let port = define_flag!("host", 50051, "The port of the largetable service");
    let limit = define_flag!("limit", 100, "The max records to return.");

    // The remaining string arguments are the query.
    let query = parse_flags!(host, limit);
    let c = largetable_client::LargeTableRemoteClient::new(&host.value(), port.value());

    let verb = query[0].as_str();
    match verb.as_ref() {
        "write" => {
            assert_eq!(query.len(), 4, "`write` expects 3 arguments");
            let res = c.write(
                query[1].as_str(),
                query[2].as_str(),
                0,
                query[3].to_owned().into_bytes(),
            );
            println!("OK (written at {})", res.get_timestamp());
        }
        "read" => {
            assert_eq!(query.len(), 3, "`read` expects 2 arguments");
            let res = c.read(&query[1], &query[2], 0);
            if res.get_found() {
                io::stdout().write(res.get_data()).unwrap();
                print!("\n");
            } else {
                eprintln!("no data");
            }
        }
        "read_scope" => {
            assert!(
                query.len() > 1,
                "`read_scope` expects at least one argument"
            );

            let row_scope = if query.len() > 2 { &query[2] } else { "" };
            let min_col = if query.len() > 3 { &query[3] } else { "" };
            let max_col = if query.len() > 4 { &query[4] } else { "" };

            let res = c.read_scoped(&query[1], row_scope, min_col, max_col, limit.value(), 0);
            for record in res.get_records() {
                print!("[{}][{}] -> ", record.get_row(), record.get_column());
                io::stdout().write(record.get_data()).unwrap();
                print!("\n");
            }
        }
        "shard_hint" => {
            let row_scope = if query.len() == 2 { "" } else { &query[2] };

            let res = c.shard_hint(&query[1], row_scope);
            for shard in res.get_shards() {
                println!("{}", shard);
            }
        }
        _ => panic!("Invalid verb: {}", verb),
    };
}

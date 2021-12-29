use fab_lib::{BuildEnvironment, FilesystemResolver, TargetIdentifier};
use std::str::FromStr;

#[macro_use]
extern crate flags;

fn usage() {
    println!("USAGE: fab <operation> <target_name>");
    std::process::exit(1);
}

fn main() {
    let args = parse_flags!();
    if args.len() != 2 {
        return usage();
    }

    let res = FilesystemResolver::new();
    let env = BuildEnvironment::new(res, std::path::PathBuf::from_str("/tmp/fab").unwrap());
    match args[0].as_str() {
        "build" => match env.build(TargetIdentifier::from_str(&args[1])) {
            Ok(_) => return,
            Err(e) => {
                eprintln!("build failed!");
                eprintln!("{}", e.message);
                std::process::exit(1);
            }
        },
        other => {
            eprintln!("unknown operation {}", other);
            std::process::exit(1);
        }
    }
}

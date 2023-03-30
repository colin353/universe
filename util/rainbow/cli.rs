fn main() {
    let args = flags::parse_flags!();

    let client = auth_client::AuthClient::new_tls("auth.colinmerkel.xyz", 8888);
    let token = cli::load_auth();
    client.global_init(token);

    match args[0].as_str() {
        "resolve" => {
            if args.len() != 2 {
                eprintln!("usage: rainbow resolve <binary_name>:<tag>");
                std::process::exit(1);
            }
            let (binary, tag) = rainbow::parse(&args[1]).unwrap();
            match rainbow::resolve_binary(binary, tag) {
                Some(b) => {
                    println!("{b}")
                }
                None => {
                    eprintln!("failed to resolve");
                    std::process::exit(1);
                }
            };
        }
        "publish" => {
            if args.len() != 3 {
                eprintln!("usage: rainbow publish <binary_name>:<tag> <path>");
                std::process::exit(1);
            }
            let (binary, tag) = rainbow::parse(&args[1]).unwrap();
            let sha = rainbow::publish(binary, &[tag], &std::path::Path::new(&args[2])).unwrap();
            println!("published as {sha}");
        }
        _ => {
            eprintln!("usage: rainbow <command> <binary_name>");
            std::process::exit(1);
        }
    }
}

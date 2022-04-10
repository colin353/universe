use grpc::ClientStubExt;
use metal_grpc_rust::MetalService;

#[macro_use]
extern crate flags;

fn update(down: bool, filename: &str, client: &metal_grpc_rust::MetalServiceClient) {
    let content = match std::fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("unable to read file {}!", filename);
            std::process::exit(1);
        }
    };

    let cfg = match config::read_config(&content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("unable to parse config file: \n\n: {:?}", e);
            std::process::exit(1);
        }
    };

    let mut req = metal_grpc_rust::UpdateRequest::new();
    req.set_config(cfg);
    req.set_down(down);

    let resp = match client.update(grpc::RequestOptions::new(), req).wait() {
        Ok(r) => r.1,
        Err(_) => {
            eprintln!("failed to connect to metal service, is the metal service running?");
            std::process::exit(1);
        }
    };

    if !resp.get_success() {
        eprintln!("operation failed: {}", resp.get_error_message());
        std::process::exit(1);
    }

    println!("{}", diff::fmt_diff(resp.get_diff_applied()));

    println!("OK");
}

fn up(args: &[String], client: &metal_grpc_rust::MetalServiceClient) {
    if args.len() != 1 {
        eprintln!("USAGE: metal [options] up [config]");
        std::process::exit(1);
    }
    update(false, args.get(0).unwrap().as_str(), client);
}

fn down(args: &[String], client: &metal_grpc_rust::MetalServiceClient) {
    if args.len() != 1 {
        eprintln!("USAGE: metal [options] down [config]");
        std::process::exit(1);
    }
    update(true, args.get(0).unwrap().as_str(), client);
}

fn usage() {
    eprintln!("USAGE: metal [options] [command] [config]");
    std::process::exit(1);
}

fn main() {
    let args = parse_flags!();

    let client =
        metal_grpc_rust::MetalServiceClient::new_plain("localhost", 20202, Default::default())
            .expect("failed to create metal gRPC client");

    match args.get(0).map(|s| s.as_str()) {
        Some("up") => up(&args[1..], &client),
        Some("down") => down(&args[1..], &client),
        _ => usage(),
    }
}

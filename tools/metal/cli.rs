use grpc::ClientStubExt;
use metal_grpc_rust::{MetalService, TaskState};

#[macro_use]
extern crate flags;

fn render_time(ts: u64) -> String {
    let mut time = ts;
    let units: &'static str;
    if ts < 60_000_000 {
        time = ts / 1_000_000;
        units = "seconds";
    } else if ts < 60 * 60_000_000 {
        time = ts / 60_000_000;
        units = "minutes";
    } else {
        time = ts / (60 * 60_000_000);
        units = "hours"
    };
    format!("{} {}", time, units)
}

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

fn status(args: &[String], client: &metal_grpc_rust::MetalServiceClient) {
    let mut req = metal_grpc_rust::StatusRequest::new();

    if args.len() > 1 {
        eprintln!("USAGE: metal [options] status [selector]");
        std::process::exit(1);
    }

    if args.len() == 1 {
        req.set_selector(args[0].to_string());
    }
    let response = match client.status(grpc::RequestOptions::new(), req).wait() {
        Ok(r) => r.1,
        Err(_) => {
            eprintln!("failed to connect to metal service, is the metal service running?");
            std::process::exit(1);
        }
    };

    // Print header
    println!("TYPE  {: <32}  STATE", "NAME");
    println!("====  {: <32}  =====", "====");

    let mut tasks: Vec<_> = response.get_tasks().iter().collect();
    tasks.sort_by_key(|t| t.get_name());

    if tasks.is_empty() {
        if args.len() == 0 {
            println!("There are no tasks running.");
        } else {
            println!("There are no running tasks with that selector.");
        }
    }

    for task in tasks {
        let status_line = match task.get_runtime_info().get_state() {
            TaskState::RUNNING => {
                let tr = render_time(core::ts() - task.get_runtime_info().get_last_start_time());
                format!(" ({})", tr)
            }
            TaskState::SUCCESS => {
                let tr = render_time(core::ts() - task.get_runtime_info().get_last_start_time());
                format!(" ({} ago)", tr)
            }
            _ => format!(" (code={})", task.get_runtime_info().get_exit_status()),
        };
        println!(
            "task  {: <32}  {:?}{}",
            task.get_name(),
            task.get_runtime_info().get_state(),
            status_line,
        )
    }
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
        Some("status") => status(&args[1..], &client),
        _ => usage(),
    }
}

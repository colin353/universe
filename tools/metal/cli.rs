use metal_bus::TaskState;

use std::convert::TryInto;
use std::sync::Arc;

#[macro_use]
extern crate flags;

fn render_time(ts: u64) -> String {
    let time: u64;
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

fn update(down: bool, filename: &str, client: &metal_bus::MetalClient) {
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

    let mut req = metal_bus::UpdateRequest::new();
    req.config = cfg;
    req.down = down;

    let resp = match client.update(req) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("failed to connect to metal service, is the metal service running?");
            std::process::exit(1);
        }
    };

    if !resp.success {
        eprintln!("operation failed: {}", resp.error_message);
        std::process::exit(1);
    }

    println!("{}", diff::fmt_diff(&resp.diff_applied));

    println!("OK");
}

fn up(args: &[String], client: &metal_bus::MetalClient) {
    if args.len() != 1 {
        eprintln!("USAGE: metal [options] up [config]");
        std::process::exit(1);
    }
    update(false, args.get(0).unwrap().as_str(), client);
}

fn down(args: &[String], client: &metal_bus::MetalClient) {
    if args.len() != 1 {
        eprintln!("USAGE: metal [options] down [config]");
        std::process::exit(1);
    }
    update(true, args.get(0).unwrap().as_str(), client);
}

fn reload(args: &[String], client: &metal_bus::MetalClient) {
    if args.len() != 1 {
        eprintln!("USAGE: metal [options] reload [config]");
        std::process::exit(1);
    }
    update(true, args.get(0).unwrap().as_str(), client);
    update(false, args.get(0).unwrap().as_str(), client);
}

fn status(args: &[String], client: &metal_bus::MetalClient) {
    let mut req = metal_bus::StatusRequest::new();

    if args.len() > 1 {
        eprintln!("USAGE: metal [options] status [selector]");
        std::process::exit(1);
    }

    if args.len() == 1 {
        req.selector = args[0].to_string();
    }
    let response = match client.status(req) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("failed to connect to metal service, is the metal service running?");
            std::process::exit(1);
        }
    };

    // Print header
    println!("TYPE  {: <32}  STATE", "NAME");
    println!("====  {: <32}  =====", "====");

    let mut tasks: Vec<_> = response.tasks.iter().collect();
    tasks.sort_by_key(|t| &t.name);

    if tasks.is_empty() {
        if args.len() == 0 {
            println!("There are no tasks running.");
        } else {
            println!("There are no running tasks with that selector.");
        }
    }

    for task in tasks {
        let status_line = match task.runtime_info.state {
            TaskState::Running => {
                let tr = render_time(core::ts() - task.runtime_info.last_start_time);
                format!(" ({})", tr)
            }
            TaskState::Success => {
                let tr = render_time(core::ts() - task.runtime_info.last_start_time);
                format!(" ({} ago)", tr)
            }
            _ => format!(" (code={})", task.runtime_info.exit_status),
        };
        println!(
            "task  {: <32}  {:?}{}",
            task.name, task.runtime_info.state, status_line,
        )
    }
}

fn logs(args: &[String], client: &metal_bus::MetalClient, all: bool, stdout: bool, stderr: bool) {
    if args.len() != 1 {
        eprintln!("USAGE: metal [options] logs [resource_name]");
        std::process::exit(1);
    }

    let mut req = metal_bus::GetLogsRequest::new();
    req.resource_name = args[0].to_string();

    let response = match client.get_logs(req) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("failed to connect to metal service, is the metal service running?");
            std::process::exit(1);
        }
    };

    let mut logs_to_render: Box<dyn Iterator<Item = &metal_bus::Logs>> =
        Box::new(std::iter::once(response.logs.last()).filter_map(|x| x));
    if all {
        logs_to_render = Box::new(response.logs.iter());
    }

    for log in logs_to_render {
        println!("{}", "=".repeat(70));
        println!("= TASK       {:<55} =", &args[0]);
        println!(
            "= STARTED    {:<55} =",
            format!("{} ago", render_time(core::ts() - log.start_time)),
        );
        if log.end_time != 0 {
            println!(
                "= RAN FOR    {:<55} =",
                render_time(log.end_time.checked_sub(log.start_time).unwrap_or(0))
            );
            println!("= EXIT CODE  {:<55} =", log.exit_status);
        }
        println!("{}\n", "=".repeat(70));

        let mut showed_logs = false;
        if stdout && log.stdout.len() > 0 {
            showed_logs = true;
            println!("{:=^1$}\n", " STDOUT ", 70);
            println!("{}", log.stdout);
        }

        if stderr && log.stderr.len() > 0 {
            showed_logs = true;
            println!("{:=^1$}\n", " STDERR ", 70);
            println!("{}", log.stderr);
        }

        if !showed_logs && (stderr || stdout) {
            println!("(no logs captured)");
        }
    }
}

fn resolve(args: &[String], client: &metal_bus::MetalClient) {
    if args.len() != 1 {
        eprintln!("USAGE: metal [options] resolve [resource_name]");
        std::process::exit(1);
    }

    let mut req = metal_bus::ResolveRequest::new();
    req.service_name = args[0].to_string();

    let response = match client.resolve(req) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("failed to connect to metal service, is the metal service running?");
            std::process::exit(1);
        }
    };

    if response.endpoints.is_empty() {
        eprintln!("failed to resolve resource");
        std::process::exit(1);
    }

    for endpoint in &response.endpoints {
        let ip: std::net::IpAddr;
        if endpoint.ip_address.len() == 4 {
            let bytes: [u8; 4] = endpoint.ip_address[0..4].try_into().unwrap();
            ip = std::net::IpAddr::from(bytes);
        } else if endpoint.ip_address.len() == 16 {
            let bytes: [u8; 16] = endpoint.ip_address[0..16].try_into().unwrap();
            ip = std::net::IpAddr::from(bytes);
        } else {
            eprintln!("invalid IP address: {:?}", endpoint.ip_address);
            continue;
        }

        println!("{}:{}", ip, endpoint.port);
    }
}

fn usage() {
    eprintln!("USAGE: metal [options] [command] [config]");
    std::process::exit(1);
}

fn main() {
    let all = define_flag!(
        "all",
        false,
        "[logs] Whether to show all logs (true) or just the most recent"
    );
    let stdout = define_flag!("stdout", true, "[logs] Whether to show the stdout log");
    let stderr = define_flag!("stderr", true, "[logs] Whether to show the stderr log");
    let host = define_flag!(
        "host",
        String::from("localhost"),
        "the metal host to connect to"
    );
    let port = define_flag!("port", 20202_u16, "the port to connect with");
    let token = define_flag!("token", String::from(""), "the authentication token to use");
    let args = parse_flags!(all, host, port, token, stdout, stderr);

    // If the host is set, but not auth token, try to pick it up from the environment
    let token = token.value();
    let connector: Arc<dyn bus::BusClient> = if host.value() != "localhost" && token.is_empty() {
        let token = cli::load_auth();
        let mut connector = bus_rpc::HyperSyncClient::new_tls(host.value(), port.value());
        connector.add_header(hyper::header::AUTHORIZATION, token);
        std::sync::Arc::new(connector)
    } else if !token.is_empty() {
        let mut connector = bus_rpc::HyperSyncClient::new_tls(host.value(), port.value());
        connector.add_header(hyper::header::AUTHORIZATION, token);
        Arc::new(connector)
    } else {
        Arc::new(bus_rpc::HyperSyncClient::new(host.value(), port.value()))
    };

    let client = metal_bus::MetalClient::new(connector);

    match args.get(0).map(|s| s.as_str()) {
        Some("up") => up(&args[1..], &client),
        Some("down") => down(&args[1..], &client),
        Some("reload") => reload(&args[1..], &client),
        Some("status") => status(&args[1..], &client),
        Some("resolve") => resolve(&args[1..], &client),
        Some("logs") => logs(
            &args[1..],
            &client,
            all.value(),
            stdout.value(),
            stderr.value(),
        ),
        _ => usage(),
    }
}

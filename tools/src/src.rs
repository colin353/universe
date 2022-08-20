use service::SrcDaemonServiceHandler;

fn usage() {
    eprintln!("usage: src <command>");
    std::process::exit(1);
}

fn init(data_dir: std::path::PathBuf, basis: String) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let basis = match core::parse_basis(&basis) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e.to_string());
            std::process::exit(1);
        }
    };

    let d = daemon_service::SrcDaemon::new(data_dir).expect("failed to initialize src!");
    let client = d
        .get_client(&basis.host)
        .expect("failed to construct client");

    let resp = match client.get_repository(service::GetRepositoryRequest {
        token: String::new(),
        owner: basis.owner.clone(),
        name: basis.name.clone(),
    }) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("couldn't reach src server!");
            std::process::exit(1);
        }
    };

    if resp.failed {
        // That's OK, just means the repo doesn't exist
    } else if resp.index != 0 {
        eprintln!("that repository already exists, and isn't empty!");
        std::process::exit(1);
    }

    let alias = d
        .initialize_repo(basis.clone(), &cwd)
        .expect("failed to initialize");
    println!(
        "initialized change {} @ {}",
        alias,
        core::fmt_basis(basis.as_view())
    );
}

fn change(_data_dir: std::path::PathBuf, name: String, basis: String) {
    let _basis = match core::parse_basis(&basis) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e.to_string());
            std::process::exit(1);
        }
    };

    todo!()
}

fn diff(data_dir: std::path::PathBuf) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = daemon_service::SrcDaemon::new(data_dir).expect("failed to initialize src!");
    let resp = d
        .diff(service::DiffRequest {
            dir: cwd
                .to_str()
                .expect("current working directory must be valid unicode!")
                .to_owned(),
            ..Default::default()
        })
        .unwrap();

    if resp.failed {
        eprintln!("{}", resp.error_message);
        std::process::exit(1);
    }

    println!("{:#?}", resp);
}

fn main() {
    let name = flags::define_flag!("name", String::new(), "the name of the change to create");
    let basis = flags::define_flag!(
        "basis",
        String::new(),
        "the basis of the change (e.g. src.colinmerkel.xyz/owner/name"
    );
    let home = std::env::var("HOME").expect("unable to detect $HOME directory!");
    let data_directory = flags::define_flag!(
        "data_directory",
        format!("{}/.src", home),
        "the data directory for src on this computer"
    );

    let args = flags::parse_flags!(name, basis);

    if args.len() == 0 {
        usage();
    }

    let data_dir = std::path::PathBuf::from(data_directory.value());

    match args[0].as_str() {
        "init" => {
            if args.len() != 2 {
                eprintln!("usage: src init <repo>");
                std::process::exit(1);
            }
            init(data_dir, args[1].clone())
        }
        "change" => {
            if args.len() != 1 {
                eprintln!("usage: src change [--name=<change name>] [--basis=<basis>]");
                std::process::exit(1);
            }
            change(data_dir, name.value(), basis.value())
        }
        "diff" => {
            if args.len() != 1 {
                eprintln!("usage: src diff");
                std::process::exit(1);
            }
            diff(data_dir)
        }
        _ => usage(),
    }
}

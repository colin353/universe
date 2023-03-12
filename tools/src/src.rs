use cli::*;

fn main() {
    let name = flags::define_flag!("name", String::new(), "the name of the change to create");
    let basis = flags::define_flag!(
        "basis",
        String::new(),
        "the basis of the change (e.g. src.colinmerkel.xyz/owner/name"
    );
    let msg = flags::define_flag!(
        "msg",
        String::new(),
        "a message to include with the snapshot (optional)"
    );
    let home = std::env::var("HOME").expect("unable to detect $HOME directory!");
    let data_directory = flags::define_flag!(
        "data_directory",
        format!("{}/.src", home),
        "the data directory for src on this computer"
    );

    let args = flags::parse_flags!(name, basis, msg, data_directory);

    let data_dir = std::path::PathBuf::from(data_directory.value());

    if args.len() == 0 {
        history(data_dir);
        return;
    }

    match args[0].as_str() {
        "create" => {
            if args.len() != 2 {
                eprintln!("usage: src create <hostname>/<owner_name>/<repo_name>");
                std::process::exit(1);
            }
            create(data_dir, args[1].clone())
        }
        "checkout" => {
            if args.len() != 2 {
                eprintln!("usage: src init <repo>");
                std::process::exit(1);
            }
            checkout(data_dir, name.value(), args[1].clone())
        }
        "diff" => {
            if args.len() != 1 {
                eprintln!("usage: src diff");
                std::process::exit(1);
            }
            diff(data_dir)
        }
        "files" => {
            if args.len() != 1 {
                eprintln!("usage: src diff");
                std::process::exit(1);
            }
            files(data_dir)
        }
        "snapshot" => {
            if args.len() != 1 {
                eprintln!("usage: src snapshot [--msg=<message>]");
                std::process::exit(1);
            }
            snapshot(data_dir, msg.value())
        }
        "submit" => {
            if args.len() != 1 {
                eprintln!("usage: src submit");
                std::process::exit(1);
            }
            submit(data_dir)
        }
        "history" => history(data_dir),
        "jump" => jump(data_dir, name.value()),
        "status" => status(data_dir),
        "push" => push(data_dir, msg.value()),
        "sync" => sync(data_dir, std::collections::HashMap::new()),
        "revert" => {
            if args.len() < 2 {
                eprintln!("usage: src revert <filename> [<filename2>, ...]");
                std::process::exit(1);
            }
            revert(data_dir, &args[1..])
        }
        "spaces" => spaces(data_dir),
        "clean" => clean(data_dir),
        "login" => {
            if args.len() != 2 {
                eprintln!("usage: src login <host>");
            }
            login(data_dir, &args[1], args.get(2).map(|s| s.as_str()))
        }
        _ => usage(),
    }
}

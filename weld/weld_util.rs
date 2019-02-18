#[macro_use]
extern crate flags;
extern crate weld;

fn usage() {
    println!("USAGE: snap <command> <filename>");
    println!("use snap --help for details.");
}

fn load_change_file(change_file: &str) -> weld::Change {
    let contents = match std::fs::read_to_string(change_file) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Unable to open file `{}`", change_file);
            std::process::exit(1);
        }
    };
    let mut c = weld::Change::new();
    weld::deserialize_change(&contents, &mut c).unwrap();
    c
}

fn main() {
    let hostname = define_flag!(
        "weld_hostname",
        String::from("localhost"),
        "The hostname of the local weld service."
    );

    let port = define_flag!(
        "port",
        8001 as u16,
        "The port to use for the local weld service"
    );

    let file = define_flag!("file", String::from(""), "The file path to refer to");
    let space = define_flag!("space", String::from(""), "The space to use");
    let change_file = define_flag!(
        "change_file",
        String::from(""),
        "A file containing a change description + annotations."
    );

    let args = parse_flags!(hostname, port, file, space, change_file); //, change_file);
    if args.len() != 1 {
        return usage();
    }

    let client = weld::WeldLocalClient::new(&hostname.value(), port.value());
    match args[0].as_ref() {
        "new" => {
            let mut change = weld::Change::new();
            change.set_friendly_name(space.value());
            let s = client.make_change(change);
            println!("created change {} @ {}", s.get_id(), s.get_based_index());
        }
        "get_change" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            let change = match maybe_id {
                Some(id) => client.get_change(weld::change(id)),
                None => {
                    eprintln!("couldn't find change, giving blank description");
                    weld::Change::new()
                }
            };
            println!("{}", weld::serialize_change(&change, true));
        }
        "cat" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            if let Some(id) = maybe_id {
                let f = client.read(weld::file_id(id, file.value(), 0));
                match f.get_found() {
                    true => println!("{}", String::from_utf8_lossy(f.get_contents())),
                    false => println!("No such file."),
                }
            } else {
                eprintln!("No such client '{}`", space.value());
                std::process::exit(1);
            }
        }
        "ls" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            if let Some(id) = maybe_id {
                let files = client.list_files(weld::file_id(id, file.value(), 0));
                for f in files {
                    println!("{}", f.get_filename());
                }
            } else {
                eprintln!("No such client '{}`", space.value());
                std::process::exit(1);
            }
        }
        "rm" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            if let Some(id) = maybe_id {
                client.delete(weld::file_id(id, file.value(), 0));
            } else {
                println!("No such client '{}`", space.value());
                std::process::exit(1);
            }
        }
        "changes" => {
            let changes = client.list_changes();
            if changes.len() == 0 {
                println!("No changes.");
            }
            for c in changes {
                eprintln!("{} @ {}", c.get_friendly_name(), c.get_based_index());
            }
        }
        "snapshot" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            if let Some(id) = maybe_id {
                let mut c = load_change_file(&change_file.value());
                c.set_id(id);
                let response = client.snapshot(c);
                println!(
                    "saved snapshot as {}@{}",
                    response.get_change_id(),
                    response.get_snapshot_id()
                );
            } else {
                eprintln!("No such client '{}`", space.value());
                std::process::exit(1);
            }
        }
        "submit" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            if let Some(id) = maybe_id {
                let mut change = weld::Change::new();
                change.set_id(id);
                let response = client.submit(change);
                println!("submitted as #{}", response.get_id());
            } else {
                eprintln!("No such client '{}`", space.value());
                std::process::exit(1);
            }
        }
        x => println!("Unknown command: {}", x),
    }
}

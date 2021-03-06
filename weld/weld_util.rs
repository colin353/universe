#[macro_use]
extern crate flags;
extern crate weld;

use std::io::Write;
use std::process::{Command, Stdio};

fn usage() {
    println!("USAGE: weld_util <command> <filename>");
    println!("use weld_util --help for details.");
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

fn edit_file(filename: &str) {
    let editor = match std::env::var("EDITOR") {
        Ok(x) => x,
        Err(_) => String::from("nano"),
    };
    Command::new(editor)
        .arg(filename)
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .output()
        .unwrap();
}

fn snapshot(client: &weld::WeldLocalClient, space: String, update_description: bool) {
    let maybe_id = client.lookup_friendly_name(space.clone());
    if let Some(id) = maybe_id {
        let change = match maybe_id {
            Some(id) => {
                let mut req = weld::GetChangeRequest::new();
                req.mut_change().set_id(id);
                client.get_change(req)
            }
            None => {
                eprintln!("couldn't find change");
                weld::Change::new()
            }
        };

        let mut c = if change.get_description().is_empty() || update_description {
            // Edit the description (if it isn't already set)
            let filename = format!("/tmp/change-{}", id);
            {
                let mut f = std::fs::File::create(&filename).unwrap();
                f.write_all(weld::serialize_change(&change, true).as_bytes())
                    .unwrap();
            }
            edit_file(&filename);
            load_change_file(&filename)
        } else {
            change
        };
        c.set_id(id);
        let response = client.snapshot(c);
        println!(
            "saved snapshot as {}@{}",
            response.get_change_id(),
            response.get_snapshot_id()
        );
    } else {
        eprintln!("No such client '{}`", space);
        std::process::exit(1);
    }
}

fn main() {
    let hostname = define_flag!(
        "weld_hostname",
        String::from("127.0.0.1"),
        "The hostname of the local weld service."
    );

    let port = define_flag!(
        "port",
        8008 as u16,
        "The port to use for the local weld service"
    );

    let file = define_flag!("file", String::from(""), "The file path to refer to");
    let space = define_flag!("space", String::from(""), "The space to use");
    let change_file = define_flag!(
        "change_file",
        String::from(""),
        "A file containing a change description + annotations."
    );
    let target = define_flag!("target", String::new(), "A bazel target to build");
    let update_description = define_flag!(
        "update_description",
        false,
        "whether to update the change description when snapshotting"
    );

    let args = parse_flags!(
        hostname,
        port,
        file,
        space,
        change_file,
        target,
        update_description
    );
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
                Some(id) => {
                    let mut req = weld::GetChangeRequest::new();
                    req.mut_change().set_id(id);
                    client.get_change(req)
                }
                None => {
                    eprintln!("couldn't find change");
                    weld::Change::new()
                }
            };
            println!("{}", weld::serialize_change(&change, true));
        }
        "basis" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            let change = match maybe_id {
                Some(id) => {
                    let mut req = weld::GetChangeRequest::new();
                    req.mut_change().set_id(id);
                    client.get_change(req)
                }
                None => {
                    eprintln!("couldn't find change");
                    std::process::exit(1);
                }
            };
            println!("{}", change.get_based_index());
        }
        "files" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            let change = match maybe_id {
                Some(id) => {
                    let mut req = weld::GetChangeRequest::new();
                    req.mut_change().set_id(id);
                    req.set_filled(true);
                    client.get_change(req)
                }
                None => {
                    eprintln!("couldn't find change");
                    weld::Change::new()
                }
            };
            for file in change.get_staged_files() {
                println!("{}", file.get_filename());
            }
        }
        "patch" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            let patch = match maybe_id {
                Some(id) => client.get_patch(weld::change(id)),
                None => {
                    println!("No such change.");
                    std::process::exit(1);
                }
            };
            println!("{}", patch);
        }
        "cat" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            if let Some(id) = maybe_id {
                let f = client.read(weld::file_id(id, file.path(), 0));
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
                let files = client.list_files(weld::file_id(id, file.path(), 0));
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
                client.delete(weld::file_id(id, file.path(), 0));
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
        "clean" => {
            let mut deleted = false;
            for change in client
                .clean_submitted_changes()
                .get_deleted_friendly_names()
            {
                deleted = true;
                println!("deleted: {}", change);
            }

            if !deleted {
                println!("no changes to delete");
            }
        }
        "revert" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            if let Some(id) = maybe_id {
                let mut req = weld::WriteRequest::new();
                req.set_id(id);
                req.mut_file().set_filename(file.path());
                req.mut_file().set_reverted(true);
                client.write(req);
            }
        }
        "snapshot" => snapshot(&client, space.value(), update_description.value()),
        "submit" => {
            let maybe_id = client.lookup_friendly_name(space.value());
            if let Some(id) = maybe_id {
                let mut change = weld::Change::new();
                change.set_id(id);
                let response = client.submit(change);
                match response.get_status() {
                    weld::SubmitStatus::OK => println!("submitted as #{}", response.get_id()),
                    weld::SubmitStatus::REQUIRES_SYNC => {
                        println!("out of date - sync required");
                        std::process::exit(1);
                    }
                    _ => {
                        println!("unknown submit error");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("No such client '{}`", space.value());
                std::process::exit(1);
            }
        }
        "query" => {
            let id = match client.lookup_friendly_name(space.value()) {
                Some(x) => x,
                None => {
                    eprintln!("No such client '{}'", space.value());
                    std::process::exit(1);
                }
            };
            let mut req = weld::GetChangeRequest::new();
            req.mut_change().set_id(id);
            let c = client.get_change(req);

            let mut req = weld::RunBuildQueryRequest::new();
            req.set_change_id(c.get_remote_id());
            let response = client.run_build_query(req);
            println!("query: {:?}", response);
        }
        "build" => {
            let id = match client.lookup_friendly_name(space.value()) {
                Some(x) => x,
                None => {
                    eprintln!("No such client '{}'", space.value());
                    std::process::exit(1);
                }
            };
            let mut req = weld::GetChangeRequest::new();
            req.mut_change().set_id(id);
            let c = client.get_change(req);

            let mut req = weld::RunBuildRequest::new();
            req.set_change_id(c.get_remote_id());
            req.set_target(target.value());
            let response = client.run_build(req);
            println!("build: {:?}", response);
        }
        "apply-patch" => {
            let id = match client.lookup_friendly_name(space.value()) {
                Some(x) => x,
                None => {
                    eprintln!("No such client '{}'", space.value());
                    std::process::exit(1);
                }
            };
            let mut req = weld::GetChangeRequest::new();
            req.mut_change().set_id(id);
            let c = client.get_change(req);

            let mut req = weld::ApplyPatchRequest::new();
            req.set_change_id(c.get_remote_id());
            let response = client.apply_patch(req);
            println!("apply patch: {:?}", response);
        }
        "delete" => {
            let id = match client.lookup_friendly_name(space.value()) {
                Some(x) => x,
                None => {
                    eprintln!("No such client '{}'", space.value());
                    std::process::exit(1);
                }
            };

            println!(
                "Are you sure you want to delete? If so, type the repo name ({}):",
                space.value()
            );
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();

            if input.trim() != space.value() {
                println!("That's not right, aborting.");
                std::process::exit(1);
            }

            let mut req = weld::Change::new();
            req.set_id(id);
            client.delete_change(req);
            println!("change deleted");
        }
        "sync" => {
            let id = match client.lookup_friendly_name(space.value()) {
                Some(x) => x,
                None => {
                    eprintln!("No such client '{}'", space.value());
                    std::process::exit(1);
                }
            };
            let mut sync_request = weld::SyncRequest::new();
            sync_request.mut_change().set_id(id);

            loop {
                let result = client.sync(sync_request.clone());
                if result.get_conflicted_files().len() == 0 {
                    println!("synced to latest (#{})", result.get_index());
                    snapshot(&client, space.value(), false);
                    break;
                }

                println!(
                    "There are {} conflicts.",
                    result.get_conflicted_files().len()
                );
                for (index, conflict) in result.get_conflicted_files().iter().enumerate() {
                    println!("Conflict: {}", conflict.get_filename());
                    let filename = format!("/tmp/conflict-{}", index);
                    {
                        let mut file = std::fs::File::create(&filename).unwrap();
                        file.write_all(conflict.get_contents()).unwrap();
                    }

                    loop {
                        println!("Resolve conflict? Edit (e), Accept (a): ");
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();

                        if input.trim() == "e" {
                            edit_file(&filename);
                        } else if input.trim() == "a" {
                            let file_bytes = std::fs::read(&filename).unwrap();
                            let mut file = conflict.clone();
                            println!(
                                "file contents: `{}`",
                                std::str::from_utf8(&file_bytes).unwrap()
                            );
                            file.set_contents(file_bytes);
                            sync_request.mut_conflicted_files().push(file);
                            break;
                        }
                    }
                }
            }
        }
        x => println!("Unknown command: {}", x),
    }
}

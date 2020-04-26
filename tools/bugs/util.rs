#[macro_use]
extern crate flags;
extern crate bugs_grpc_rust as bugs;

static TEMPLATE: &str = include_str!("template.txt");

fn usage() {
    println!("USAGE: b <command>");
    println!("use b --help for details.");
}

fn parse_bug(input: &str) -> Result<bugs::Bug, String> {
    let desc = cli::Description::from_str(&input);
    let mut b = bugs::Bug::new();

    if desc.title.trim().is_empty() {
        return Err(format!("You must provide a bug title"));
    }

    for (tag, value) in &desc.tags {
        if tag == "TAGS" {
            for val in value.split(",") {
                b.mut_tags().push(val.to_owned());
            }
        } else if tag == "STATUS" {
            let trimmed_value = value.trim().to_ascii_lowercase();
            if trimmed_value.starts_with("wait") {
                b.set_status(bugs::BugStatus::WAITING);
            } else if trimmed_value == "ipr"
                || trimmed_value == "in_progress"
                || trimmed_value == "in progress"
            {
                b.set_status(bugs::BugStatus::IN_PROGRESS);
            } else if trimmed_value == "closed" || trimmed_value == "done" {
                b.set_status(bugs::BugStatus::CLOSED);
            } else {
                return Err(format!("Unknown status: {}", value));
            }
        }
    }

    b.set_title(desc.title);
    b.set_description(desc.description);

    Ok(b)
}

fn serialize_bug(input: &bugs::Bug) -> String {
    let mut d = cli::Description::new();
    d.title = input.get_title().to_owned();
    d.description = input.get_description().to_owned();
    d.tags
        .push((String::from("STATUS"), format!("{:?}", input.get_status())));
    if input.get_tags().len() > 0 {
        d.tags
            .push((String::from("TAGS"), input.get_tags().to_owned().join(",")));
    }

    d.to_string()
}

fn main() {
    let bug_hostname = define_flag!(
        "bug_hostname",
        String::from("bugs.colinmerkel.xyz"),
        "the hostname for bugs service"
    );
    let bug_port = define_flag!("bug_port", 9999, "the port for bugs service");
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth.colinmerkel.xyz"),
        "the hostname for auth service"
    );
    let auth_port = define_flag!("auth_port", 8888, "the port for auth service");

    let args = parse_flags!(bug_hostname, bug_port, auth_hostname, auth_port);

    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());
    let token = cli::load_and_check_auth(auth);

    let client = bug_client::BugClient::new_tls(&bug_hostname.value(), bug_port.value(), token);

    if args.len() == 0 {
        // By default, just list the bugs
        let mut has_in_progress = false;
        for bug in client.get_bugs(bugs::BugStatus::IN_PROGRESS).unwrap() {
            if !has_in_progress {
                println!("[IN PROGRESS]");
            }
            has_in_progress = true;
            println!("b/{} {}", bug.get_id(), bug.get_title());
        }
        if has_in_progress {
            println!("");
        }

        println!("[WAITING]");
        for bug in client.get_bugs(bugs::BugStatus::WAITING).unwrap() {
            println!("b/{} {}", bug.get_id(), bug.get_title());
        }
        return;
    }

    match args[0].as_ref() {
        "new" => {
            let mut serialized_bug = TEMPLATE.to_string();
            let mut bug = bugs::Bug::new();

            loop {
                serialized_bug = match cli::edit_string(&serialized_bug) {
                    Ok(s) => s,
                    Err(_) => {
                        eprintln!("That didn't work. Quitting");
                        std::process::exit(1);
                    }
                };

                bug = match parse_bug(&serialized_bug) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("Unable to parse bug: {}", e);
                        eprintln!("Press enter to retry editing...");
                        cli::wait_for_enter();
                        continue;
                    }
                };
                break;
            }

            let bug = client.create_bug(bug).unwrap();

            println!("created b/{}", bug.get_id());
        }
        "edit" => {
            if args[1].starts_with("b/") {
                let id = args[1][2..].parse().unwrap();

                let mut bug = match client.get_bug(id).unwrap() {
                    Some(b) => b,
                    None => {
                        eprintln!("No such bug!");
                        std::process::exit(1);
                    }
                };

                let mut serialized_bug = serialize_bug(&bug);

                loop {
                    serialized_bug = match cli::edit_string(&serialized_bug) {
                        Ok(s) => s,
                        Err(_) => {
                            eprintln!("That didn't work. Quitting");
                            std::process::exit(1);
                        }
                    };

                    bug = match parse_bug(&serialized_bug) {
                        Ok(b) => b,
                        Err(e) => {
                            eprintln!("Unable to parse bug: {}", e);
                            eprintln!("Press enter to retry editing...");
                            cli::wait_for_enter();
                            continue;
                        }
                    };
                    break;
                }

                bug.set_id(id);
                client.update_bug(bug).unwrap();
            }
        }
        _ => usage(),
    }
}

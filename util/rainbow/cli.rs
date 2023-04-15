fn main() {
    let args = flags::parse_flags!();

    let client = auth_client::AuthClient::new_tls("auth.colinmerkel.xyz", 8888);
    let token = cli::load_auth();
    client.global_init(token);

    if args.len() == 0 {
        eprintln!("usage: rainbow <command>");
        std::process::exit(1);
    }

    match args[0].as_str() {
        "resolve" => {
            if args.len() != 2 {
                eprintln!("usage: rainbow resolve <binary_name>:<tag>");
                std::process::exit(1);
            }
            let (binary, tag) = rainbow::parse(&args[1]).unwrap();
            match rainbow::resolve(binary, tag) {
                Some(b) => {
                    println!("{b}")
                }
                None => {
                    eprintln!("failed to resolve");
                    std::process::exit(1);
                }
            };
        }
        "history" => {
            if args.len() != 2 {
                eprintln!("usage: rainbow history <binary_name>:<tag>");
                std::process::exit(1);
            }

            let (binary, tag) = rainbow::parse(&args[1]).unwrap();
            let tag_log = rainbow::TagLog::for_tag(binary, tag).unwrap();

            for (idx, (ts, target)) in tag_log.entries.iter().enumerate().rev() {
                let (duration, suffix) = if idx == tag_log.entries.len() - 1 {
                    (time::timestamp() - *ts, " [current]")
                } else {
                    (tag_log.entries[idx + 1].0 - *ts, "")
                };
                let timestr = format!(
                    "{} for {}",
                    time::fmt_timestamp(*ts),
                    time::fmt_duration(std::time::Duration::from_secs(duration)),
                );
                println!(
                    "{timestr:30} ...{}{suffix}",
                    &target[std::cmp::max(0, target.len() as isize - 24) as usize..]
                )
            }
        }
        "revert" => {
            if args.len() != 2 {
                eprintln!("usage: rainbow revert <binary_name>:<tag>");
                std::process::exit(1);
            }

            let (binary, tag) = rainbow::parse(&args[1]).unwrap();
            let tag_log = rainbow::TagLog::for_tag(binary, tag).unwrap();

            if tag_log.entries.len() <= 1 {
                println!("not enough entries to revert");
                std::process::exit(1);
            }

            let choices: Vec<_> = tag_log
                .entries
                .iter()
                .enumerate()
                .rev()
                .map(|(idx, (ts, target))| {
                    let (duration, suffix) = if idx == tag_log.entries.len() - 1 {
                        (time::timestamp() - *ts, " [current]")
                    } else {
                        (tag_log.entries[idx + 1].0 - *ts, "")
                    };

                    let timestr = format!(
                        "{} for {}",
                        time::fmt_timestamp(*ts),
                        time::fmt_duration(std::time::Duration::from_secs(duration)),
                    );
                    format!(
                        "{timestr:30} ...{}{suffix}",
                        &target[std::cmp::max(0, target.len() as isize - 24) as usize..]
                    )
                })
                .collect();

            let choice = sel::select(choices);

            if let Some(idx) = choice {
                let idx = tag_log.entries.len() - 1 - idx;

                rainbow::update_tag(binary, tag, tag_log.entries[idx].1.clone()).unwrap();
                println!("reverted to {:?}", tag_log.entries[idx].1);
            } else {
                std::process::exit(1);
            }
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

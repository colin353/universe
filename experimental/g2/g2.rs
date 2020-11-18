use g2_proto_rust::*;
use git::GitError;
use recordio::{RecordIOReader, RecordIOWriter};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::BufRead;

#[macro_use]
extern crate flags;

fn get_g2_config_dir() -> Result<String, GitError> {
    let root_dir = git::get_root_directory()?;
    let mut h = DefaultHasher::new();
    root_dir.hash(&mut h);

    let home = std::env::var("HOME").unwrap();
    Ok(format!("{}/.g2/{}", home, h.finish()))
}

fn usage() {
    eprintln!("use g2 --help for details.");
    std::process::exit(1);
}

fn readline() -> String {
    for line in std::io::stdin().lock().lines() {
        let line = match line {
            Ok(x) => x,
            Err(_) => return String::new(),
        };
        return line.trim().to_string();
    }
    return String::new();
}

fn load_branches() -> Result<Vec<BranchConfig>, GitError> {
    let config_dir = get_g2_config_dir()?;
    let f = match std::fs::File::open(&format!("{}/config", config_dir)) {
        Ok(f) => f,
        Err(_) => return Err(GitError::NotConfigured),
    };

    let buf = std::io::BufReader::new(f);
    let reader = RecordIOReader::new(buf);
    let mut output = Vec::new();
    for cfg in reader {
        output.push(cfg);
    }
    Ok(output)
}

fn configure() {
    eprint!("What's the name of the main branch you develop from? [default=master] ");
    let mut main_branch = readline();
    if main_branch.is_empty() {
        main_branch = String::from("master");
    }

    let config_dir = match get_g2_config_dir() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("g2 must be run from within a git repository");
            std::process::exit(1);
        }
    };
    std::fs::create_dir_all(&config_dir);
    let f = std::fs::File::create(&format!("{}/config", config_dir)).unwrap();
    let mut w = RecordIOWriter::new(f);
    let mut config = BranchConfig::new();
    config.set_name(main_branch);
    w.write(&config);
    eprintln!("✔️ configured g2");
}

fn update_branches(existing_branches: &mut Vec<BranchConfig>, new_config: BranchConfig) {
    // Check whether the branch exists, and if so, update it
    let mut existing_index = None;
    for (idx, config) in existing_branches.iter().enumerate() {
        if config.get_name() == new_config.get_name() {
            existing_index = Some(idx);
            break;
        }
    }

    if let Some(idx) = existing_index {
        existing_branches[idx] = new_config;
    } else {
        existing_branches.push(new_config);
    }

    let config_dir = get_g2_config_dir().unwrap();
    let f = std::fs::File::create(&format!("{}/config", config_dir)).unwrap();
    let mut w = RecordIOWriter::new(f);
    for config in existing_branches {
        w.write(config);
    }
}

fn new_branch(main_branch: &str, branch_name: &str) -> Result<(), GitError> {
    if git::check_branch_exists(branch_name)? {
        eprint!("this branch already exists, go there instead? [Y/n] ");
        match readline().as_str() {
            "n" => std::process::exit(1),
            _ => return git::checkout(branch_name, false),
        }
    }
    git::checkout(main_branch, false)?;
    git::pull()?;
    git::checkout(branch_name, true)
}

fn run_command(mut branches: Vec<BranchConfig>, args: &[String]) -> Result<(), GitError> {
    let main_branch = branches
        .iter()
        .filter(|c| c.get_is_root_branch())
        .map(|c| c.get_name())
        .next()
        .unwrap_or("master");

    let current_branch = git::get_branch_name()?;
    let current_branch_config = branches
        .iter()
        .filter(|c| c.get_name() == current_branch)
        .next();

    match args[0].as_str() {
        "new" => {
            if args.len() != 2 {
                eprintln!("❌`g2 new` expects one argument, the branch name");
                std::process::exit(1);
            }
            new_branch(main_branch, &args[1])?;
            let mut b = BranchConfig::new();
            b.set_name(args[1].clone());
            update_branches(&mut branches, b);
        }
        "switch" | "s" => {
            for branch in branches {
                println!("{:?}", branch);
            }
        }
        "sync" => {
            git::add_all()?;
            git::commit()?;
            git::checkout(main_branch, false)?;
            git::pull()?;
            git::checkout(&current_branch, false)?;
            git::merge(main_branch)?;
        }
        "diff" => {
            if args.len() == 2 {
                git::diff_file(&current_branch, &main_branch, &args[1]);
            } else {
                git::diff(&current_branch, &main_branch);
            }
        }
        "files" => {
            for file in git::files(&current_branch, &main_branch)? {
                println!("{}", file);
            }
        }
        "upload" | "u" => {
            git::add_all();
            git::commit();
            git::push();
            if git::check_for_pr().is_none() {
                git::create_pull_request();
            }

            let config = match current_branch_config {
                Some(x) => x.clone(),
                None => {
                    let mut b = BranchConfig::new();
                    b.set_name(current_branch);
                    b.set_pull_request_url(git::check_for_pr().unwrap());
                    b
                }
            };
            update_branches(&mut branches, config);
        }
        _ => {
            eprintln!("❌unknown command `{}`", args[0]);
        }
    }

    Ok(())
}

fn main() {
    let branches = match load_branches() {
        Ok(x) => x,
        Err(_) => {
            eprint!("g2 is not yet configured, configure it? [y/N] ");
            let result = readline();
            if &result.to_lowercase() != "y" {
                std::process::exit(1);
            }

            configure();
            load_branches().unwrap()
        }
    };

    let args = parse_flags!();

    if args.len() == 0 {
        eprintln!("✔️ g2 is ready to go");
        return;
    }

    run_command(branches, args.as_slice()).unwrap();
}

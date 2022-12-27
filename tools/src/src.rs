use rand::Rng;

mod termui;

const DEFAULT_CHANGE_DESCRIPTION: &str = "

# Write description above. Lines starting with # will be ignored.
# Add annotations, e.g.
#
# R=xyz
#
# to set special fields.";

fn usage() {
    eprintln!("usage: src <command>");
    std::process::exit(1);
}

fn create(data_dir: std::path::PathBuf, basis: String) {
    let basis = match core::parse_basis(&basis) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e.to_string());
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
    let client = d
        .get_client(&basis.host)
        .expect("failed to construct client");

    let resp = match client.create(service::CreateRequest {
        token: String::new(),
        name: basis.name.clone(),
    }) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("couldn't reach src server!");
            std::process::exit(1);
        }
    };

    if resp.failed {
        eprintln!("failed to create repository: {:?}", resp.error_message);
        std::process::exit(1);
    }

    println!("OK, created {}", core::fmt_basis(basis.as_view()));
}

fn init(data_dir: std::path::PathBuf, basis: String) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let mut basis = match core::parse_basis(&basis) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e.to_string());
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
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

    if basis.change == 0 && basis.index == 0 {
        basis.index = resp.index;
    }

    let alias: String = if resp.failed {
        // That's OK, just means the repo doesn't exist
        d.initialize_repo(basis.clone(), &cwd)
            .expect("failed to initialize")
    } else {
        let alias = d.find_unused_alias(&basis.name);
        if let Err(e) = d.new_space(service::NewSpaceRequest {
            dir: cwd.to_str().unwrap().to_owned(),
            basis: basis.clone(),
            alias: alias.clone(),
        }) {
            eprintln!("failed to initialize repo!: {:?}", e);
            std::process::exit(1);
        }
        alias
    };

    println!(
        "initialized change {} @ {}",
        alias,
        core::fmt_basis(basis.as_view())
    );
}

fn new(data_dir: std::path::PathBuf, name: String, basis: String) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let mut basis = match core::parse_basis(&basis) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("unable to parse basis: {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
    let resp = match d.new_space(service::NewSpaceRequest {
        dir: cwd
            .to_str()
            .expect("current working directory must be valid unicode")
            .to_owned(),
        basis: basis.clone(),
        alias: name.clone(),
    }) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("failed to create new space: {:?}", e);
            std::process::exit(1);
        }
    };

    if resp.failed {
        eprintln!("{}", resp.error_message);
        std::process::exit(1);
    }

    basis.index = resp.index;

    println!(
        "created space {} @ {}",
        name,
        core::fmt_basis(basis.as_view())
    );
}

fn diff(data_dir: std::path::PathBuf) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
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

    // Collect up the original versions of the files to print the patch
    let metadata = match d.get_metadata(resp.basis.as_view()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let _files_and_originals: Vec<_> = resp
        .files
        .iter()
        .map(|f| {
            let original = metadata.get(&f.path).map(|fv| {
                let data = match d.get_blob(fv.get_sha()) {
                    Some(o) => o,
                    None => {
                        eprintln!("failed to get blob {:?}", core::fmt_sha(fv.get_sha()));
                        std::process::exit(1);
                    }
                };

                service::Blob {
                    sha: fv.get_sha().to_owned(),
                    data,
                }
            });
            (f, original)
        })
        .collect();

    let diff_ingredients: Vec<_> = _files_and_originals
        .iter()
        .map(|(f, o)| (*f, o.as_ref()))
        .collect();

    print!(
        "{}",
        core::render::print_patch("", "", diff_ingredients.as_slice())
    );
}

fn files(data_dir: std::path::PathBuf) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
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

    for file in &resp.files {
        println!("{}", file.path);
    }
}

fn snapshot(data_dir: std::path::PathBuf, msg: String) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
    let resp = d
        .snapshot(service::SnapshotRequest {
            dir: cwd
                .to_str()
                .expect("current working directory must be valid unicode!")
                .to_owned(),
            message: msg,
            ..Default::default()
        })
        .unwrap();

    if resp.failed {
        eprintln!("{}", resp.error_message);
        std::process::exit(1);
    }

    println!("saved snapshot @ {}", resp.timestamp);
}

fn jump(data_dir: std::path::PathBuf, name: String) {
    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");

    let change: service::Space = if !name.is_empty() {
        match d.get_change_by_alias(&name) {
            Some(c) => c,
            None => std::process::exit(1),
        }
    } else {
        let out = match d.list_changes() {
            Ok(o) => o,
            Err(_) => std::process::exit(1),
        };
        let (_, ch) = match termui::choose_space(out) {
            Some(o) => o,
            None => std::process::exit(1),
        };
        ch
    };

    std::fs::write("/tmp/jump-destination", &change.directory).unwrap();
    std::process::exit(3);
}

fn history(data_dir: std::path::PathBuf) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
    let alias = match d.get_change_alias_by_dir(&cwd) {
        Some(a) => a,
        None => {
            eprintln!("current directory is not a src directory!");
            std::process::exit(1);
        }
    };

    let snapshots = d.list_snapshots(&alias).expect("failed to list snapshots!");
    let term = tui::Terminal::new();
    for snapshot in snapshots {
        let time = core::fmt_time(snapshot.timestamp);
        let msg = if !snapshot.message.is_empty() {
            snapshot.message.as_str()
        } else {
            "snapshot"
        };

        term.set_underline();
        eprint!("{}", time);
        term.set_normal();
        eprint!("\t{} ", msg);
        term.set_grey();
        eprint!("({})\n", snapshot.timestamp);
        term.set_normal();
    }
}

fn status(data_dir: std::path::PathBuf) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
    let alias = match d.get_change_alias_by_dir(&cwd) {
        Some(a) => a,
        None => {
            eprintln!("current directory is not a src directory!");
            std::process::exit(1);
        }
    };

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
        eprintln!("failed to diff: {}", resp.error_message);
        std::process::exit(1);
    }

    match d
        .get_latest_snapshot(&alias)
        .expect("failed to get latest snapshot")
    {
        Some(s) => {
            let patch_diff = core::patch_diff(&s.files, &resp.files);
            if patch_diff.is_empty() {
                println!(
                    "Up to date with most recent snapshot ({})",
                    core::fmt_time(s.timestamp)
                );
            } else {
                println!(
                    "Changes since most recent snapshot ({}):",
                    core::fmt_time(s.timestamp)
                );
                core::render::print_diff(&patch_diff);
            }
        }
        None => {
            if resp.files.is_empty() {
                println!("No changes");
            } else {
                println!("Changes:");
                core::render::print_diff(&resp.files);
            }
        }
    };
}

pub fn edit_string(input: &str) -> Result<String, ()> {
    let editor = match std::env::var("EDITOR") {
        Ok(x) => x,
        Err(_) => String::from("nano"),
    };
    let filename = format!("/tmp/{}", rand::thread_rng().gen::<u64>());
    std::fs::write(&filename, input).unwrap();

    let output = match std::process::Command::new(&editor)
        .arg(&filename)
        .stdout(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::inherit())
        .output()
    {
        Ok(out) => out,
        Err(_) => {
            println!("unable to start editor: {}", editor);
            return Err(());
        }
    };

    if !output.status.success() {
        return Err(());
    }

    std::fs::read_to_string(&filename).map_err(|_| ())
}

fn update(data_dir: std::path::PathBuf) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");

    let alias = match d.get_change_alias_by_dir(&cwd) {
        Some(a) => a,
        None => {
            eprintln!("current directory is not a src directory!");
            std::process::exit(1);
        }
    };

    // First, check whether the current directory is associated with a remote change already. If
    // not, we have to set the description and push it.
    let mut space = match d.get_change_by_alias(&alias) {
        Some(s) => s,
        None => {
            eprintln!("current directory is not a src directory!");
            std::process::exit(1);
        }
    };

    let mut change = service::Change::new();
    change.repo_name = space.basis.name.clone();
    change.repo_owner = space.basis.owner.clone();
    change.id = space.change_id;
    if space.change_id == 0 {
        // Get the description
        match edit_string(DEFAULT_CHANGE_DESCRIPTION) {
            Ok(s) => change.description = core::normalize_change_description(&s),
            Err(_) => {
                eprintln!("update cancelled");
                std::process::exit(1);
            }
        };
    }

    // Always run a snapshot before update
    let resp = match d.snapshot(service::SnapshotRequest {
        dir: cwd
            .to_str()
            .expect("current working directory must be valid unicode!")
            .to_owned(),
        message: format!(
            "push to {}/{}/{}",
            space.basis.host, space.basis.owner, space.basis.name
        ),
        skip_if_no_changes: true,
        ..Default::default()
    }) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("couldn't reach src server!");
            std::process::exit(1);
        }
    };

    if resp.failed {
        eprintln!("failed to snapshot, {}!", resp.error_message);
        std::process::exit(1);
    }

    let snapshot = match d.get_latest_snapshot(&alias) {
        Ok(Some(s)) => s,
        _ => {
            eprintln!("no snapshot to transmit!");
            std::process::exit(1);
        }
    };

    let client = d
        .get_client(&space.basis.host)
        .expect("failed to construct client");

    let resp = match client.update_change(service::UpdateChangeRequest {
        token: String::new(),
        change: change,
        snapshot,
    }) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("couldn't reach src server!");
            std::process::exit(1);
        }
    };

    if resp.failed {
        eprintln!("update failed! {:?}", resp.error_message);
        std::process::exit(1);
    }

    // Update the local space data with the associated change ID
    space.change_id = resp.id;
    if let Err(e) = d.set_change_by_alias(&alias, &space) {
        eprintln!("failed to update local space: {:?}", e);
        std::process::exit(1);
    }
}

fn submit(data_dir: std::path::PathBuf) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");

    let alias = match d.get_change_alias_by_dir(&cwd) {
        Some(a) => a,
        None => {
            eprintln!("current directory is not a src directory!");
            std::process::exit(1);
        }
    };

    // First, check whether the current directory is associated with a remote change already. If
    // not, we have to set the description and push it.
    let space = match d.get_change_by_alias(&alias) {
        Some(s) => s,
        None => {
            eprintln!("current directory is not a src directory!");
            std::process::exit(1);
        }
    };

    if space.change_id == 0 {
        eprintln!("no remote change exists!");
        std::process::exit(1);
    }

    let snapshot = match d.get_latest_snapshot(&alias) {
        Ok(Some(s)) => s,
        _ => {
            eprintln!("no snapshot to submit!");
            std::process::exit(1);
        }
    };

    let client = d
        .get_client(&space.basis.host)
        .expect("failed to construct client");
    let resp = match client.submit(service::SubmitRequest {
        token: String::new(),
        repo_owner: space.basis.owner.clone(),
        repo_name: space.basis.name.clone(),
        change_id: space.change_id,
        snapshot_timestamp: snapshot.timestamp,
    }) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("couldn't reach src server!");
            std::process::exit(1);
        }
    };

    if resp.failed {
        eprintln!("submit failed! {:?}", resp.error_message);
        std::process::exit(1);
    }
    println!("submitted as {}", resp.index);
}

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
        "init" => {
            if args.len() != 2 {
                eprintln!("usage: src init <repo>");
                std::process::exit(1);
            }
            init(data_dir, args[1].clone())
        }
        "new" => {
            if args.len() != 1 {
                eprintln!("usage: src new [--name=<change name>] [--basis=<basis>]");
                std::process::exit(1);
            }
            new(data_dir, name.value(), basis.value())
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
        "update" => update(data_dir),
        _ => usage(),
    }
}

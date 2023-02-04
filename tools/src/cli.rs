use rand::Rng;

mod termui;

const DEFAULT_CHANGE_DESCRIPTION: &str = "

# Write description above. Lines starting with # will be ignored.
# Add annotations, e.g.
#
# R=xyz
#
# to set special fields.";

pub fn usage() {
    eprintln!("usage: src <command>");
    std::process::exit(1);
}

pub fn create(data_dir: std::path::PathBuf, basis: String) {
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

pub fn checkout(data_dir: std::path::PathBuf, name: String, arg0: String) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("unable to determine current working directory! {:?}", e);
            std::process::exit(1);
        }
    };

    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");

    let mut alias = name;
    let existing_space = d.get_change_by_alias(&arg0);
    let basis = match &existing_space {
        Some(space) => {
            alias = arg0;
            space.basis.clone()
        }
        None => {
            let mut basis = match core::parse_basis(&arg0) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("unable to parse basis: {:?}", e);
                    std::process::exit(1);
                }
            };

            if alias.is_empty() {
                alias = basis.name.clone();
            }

            // If the basis index is zero, we should checkout the latest change.
            if basis.index == 0 {
                let client = d.get_client(&basis.host).unwrap();
                let repo = match client.get_repository(service::GetRepositoryRequest {
                    token: String::new(),
                    owner: basis.owner.clone(),
                    name: basis.name.clone(),
                    ..Default::default()
                }) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("failed to checkout: {:?}", e);
                        std::process::exit(1);
                    }
                };
                basis.index = repo.index;
            }

            basis
        }
    };

    let directory = match d.checkout(cwd.clone(), basis.as_view()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("failed to checkout: {:?}", e);
            std::process::exit(1);
        }
    };

    // Apply the latest snapshot (if one exists)
    if existing_space.is_some() {
        if let Ok(Some(snapshot)) = d.get_latest_snapshot(&alias) {
            if let Err(e) = d.apply_snapshot(&cwd, basis.as_view(), &snapshot.files) {
                eprintln!("failed to apply snapshot: {:?}", e);
                std::process::exit(1);
            }
        }
    }

    match &existing_space {
        // Need to mark the space as attached
        Some(s) => {
            let mut space = s.clone();
            space.directory = directory.to_str().unwrap().to_owned();
            d.set_change_by_alias(&alias, &space).unwrap();
        }
        // Need to create a new space
        None => {
            let space = service::Space {
                directory: directory.to_str().unwrap().to_owned(),
                basis: basis.clone(),
                ..Default::default()
            };
            alias = d.find_unused_alias(&alias);
            d.set_change_by_alias(&alias, &space).unwrap();
        }
    }

    println!(
        "{} space {} @ {}",
        if existing_space.is_some() {
            "attached"
        } else {
            "created"
        },
        alias,
        core::fmt_basis(basis.as_view())
    );
}

pub fn diff(data_dir: std::path::PathBuf) {
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

    print!("{}", core::render::print_patch(diff_ingredients.as_slice()));
}

pub fn files(data_dir: std::path::PathBuf) {
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

pub fn snapshot(data_dir: std::path::PathBuf, msg: String) {
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

pub fn jump(data_dir: std::path::PathBuf, name: String) {
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

pub fn history(data_dir: std::path::PathBuf) {
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

pub fn status(data_dir: std::path::PathBuf) {
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

fn edit_string(input: &str) -> Result<String, ()> {
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

pub fn push(data_dir: std::path::PathBuf, description: String) {
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
        if !description.is_empty() {
            change.description = description;
        } else {
            // Get the description
            match edit_string(DEFAULT_CHANGE_DESCRIPTION) {
                Ok(s) => change.description = core::normalize_change_description(&s),
                Err(_) => {
                    eprintln!("update cancelled");
                    std::process::exit(1);
                }
            };
        }
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

pub fn submit(data_dir: std::path::PathBuf) {
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

pub fn revert(data_dir: std::path::PathBuf, paths: &[String]) {
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

    let space = match d.get_change_by_alias(&alias) {
        Some(s) => s,
        None => {
            eprintln!("current directory is not a src directory!");
            std::process::exit(1);
        }
    };

    let metadata = match d.get_metadata(space.basis.as_view()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "unable to get metadata for {:?}: {:?}",
                core::fmt_basis(space.basis.as_view()),
                e
            );
            std::process::exit(1);
        }
    };

    // TODO: somehow refactor checkout(...) to do this as well
    for path in paths {
        let path = cwd.join(path);
        let normalized = path
            .strip_prefix(&space.directory)
            .expect("must specify a path within the current space!");
        d.revert(normalized, &metadata).unwrap();
    }
}

pub fn sync(_data_dir: std::path::PathBuf) {
    unimplemented!()
}

pub fn spaces(data_dir: std::path::PathBuf) {
    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");
    let mut attached_spaces = Vec::new();
    let mut detached_spaces = Vec::new();
    for (alias, space) in d.get_spaces() {
        let snapshot = match d.get_latest_snapshot(&alias) {
            Ok(s) => s,
            _ => None,
        };
        if space.directory.is_empty() {
            detached_spaces.push((alias, space, snapshot));
        } else {
            attached_spaces.push((alias, space, snapshot));
        }
    }

    attached_spaces.sort_by_key(|(_, _, snapshot)| {
        std::cmp::Reverse(match snapshot {
            Some(s) => s.timestamp,
            None => 0,
        })
    });
    detached_spaces.sort_by_key(|(_, _, snapshot)| {
        std::cmp::Reverse(match snapshot {
            Some(s) => s.timestamp,
            None => 0,
        })
    });

    let term = tui::Terminal::new();

    term.set_underline();
    eprint!("space");
    term.set_normal();
    eprint!("\t\t     ");
    term.set_underline();
    eprint!("basis");
    term.set_normal();
    eprint!("\t\t\t\t      ");
    term.set_underline();
    eprint!("last modified");
    term.set_normal();
    eprint!("      ");
    term.set_underline();
    eprint!("directory\n");
    term.set_normal();
    for (alias, space, snapshot) in attached_spaces {
        eprint!(
            "{:<20.24} {:<40.40} ",
            alias,
            core::fmt_basis(space.basis.as_view()),
        );
        let time = match snapshot {
            Some(s) => core::fmt_time(s.timestamp),
            None => {
                term.set_grey();
                "no changes".to_string()
            }
        };
        eprint!("{:<15.15} ", time,);
        term.set_normal();
        eprintln!("   {}", space.directory);
    }

    for (alias, space, snapshot) in detached_spaces {
        eprint!(
            "{:<20.24} {:<40.40} ",
            alias,
            core::fmt_basis(space.basis.as_view()),
        );
        let time = match snapshot {
            Some(s) => core::fmt_time(s.timestamp),
            None => {
                term.set_grey();
                "no changes".to_string()
            }
        };
        eprint!("{:<15.15} ", time,);
        term.set_grey();
        eprintln!("   detached");
        term.set_normal();
    }
}

pub fn clean(data_dir: std::path::PathBuf) {
    let d = src_lib::Src::new(data_dir).expect("failed to initialize src!");

    let mut empty_no_changes = 0;
    let mut already_submitted = 0;
    let mut alias_desync = 0;

    for (alias, mut space) in d.get_spaces() {
        let snapshot = match d.get_latest_snapshot(&alias) {
            Ok(s) => s,
            _ => None,
        };

        // If not linked to a directoy and contains no changes, delete it
        if snapshot.is_none() && space.directory.is_empty() {
            empty_no_changes += 1;
            std::fs::remove_file(d.get_change_metadata_path(&alias)).unwrap();
        }

        // Not yet linked to a remote change, skip
        if space.change_id == 0 {
            continue;
        }

        let client = d
            .get_client(&space.basis.host)
            .expect("failed to construct client");
        let resp = match client.get_change(service::GetChangeRequest {
            token: String::new(),
            repo_owner: space.basis.owner.clone(),
            repo_name: space.basis.name.clone(),
            id: space.change_id,
        }) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if resp.failed {
            continue;
        }

        // Check that the by_dir link matches the by_alias link. If not, unattach the space.
        if !space.directory.is_empty() {
            let path = d.get_change_dir_path(&std::path::Path::new(&space.directory));
            let matching = match std::fs::read_to_string(path) {
                Ok(a) => a == alias,
                Err(_) => false,
            };
            if !matching {
                alias_desync += 1;
                space.directory = String::new();
                d.set_change_by_alias(&alias, &space).unwrap();
            }
        }

        // The change was submitted, delete it
        if resp.change.status == service::ChangeStatus::Submitted
            || resp.change.status == service::ChangeStatus::Archived
        {
            already_submitted += 1;
            std::fs::remove_file(d.get_change_metadata_path(&alias)).unwrap();
            if !space.directory.is_empty() {
                std::fs::remove_file(
                    d.get_change_dir_path(&std::path::Path::new(&space.directory)),
                )
                .unwrap();

                // TODO: Delete the linked directory as well?
                // std::fs::remove_dir_all(space.directory).unwrap();
            }
        }
    }

    if already_submitted > 0 {
        println!("removed {} spaces that were submitted", already_submitted);
    }
    if alias_desync > 0 {
        println!(
            "fixed {} spaces that were in an invalid state",
            alias_desync
        );
    }
    if empty_no_changes > 0 {
        println!(
            "removed {} spaces that were had no changes and were detached",
            empty_no_changes
        );
    }

    if already_submitted == 0 && empty_no_changes == 0 && alias_desync == 0 {
        println!("nothing to do");
    }
}

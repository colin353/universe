const DATA_DIR: &'static str = "/tmp/src_integration/src";

async fn in_cleanroom(f: impl FnOnce() + Sync + Send + 'static) -> bool {
    std::fs::remove_dir_all("/tmp/src_integration").ok();
    std::fs::create_dir_all("/tmp/src_integration/server").unwrap();
    std::fs::create_dir_all(DATA_DIR).unwrap();
    std::fs::create_dir_all("/tmp/src_integration/spaces").unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();

    let service = server_service::SrcServer::new(
        std::path::PathBuf::from("/tmp/src_integration/server"),
        String::from("localhost:44959"),
    )
    .unwrap();

    std::thread::spawn(move || {
        f();
        tx.send(()).unwrap();
    });

    let handler = std::sync::Arc::new(service);
    tokio::select! {
        _ = bus_rpc::serve(44959, service::SrcServerService(handler)) => {
            assert!(false, "Service failed to start");
            return false;
        }
        val = rx => {
            match val {
                Ok(_) => {
                    println!("tests finished");
                    return true;
                }
                Err(_) => {
                    println!("tests failed");
                    return false;
                }
            }
        }
    }
}

fn write_files(root: &std::path::Path, desc: &str) -> std::io::Result<()> {
    let mut iter = desc.lines().map(|l| l.trim()).peekable();
    while let Some(line) = iter.peek() {
        let line = line.trim();
        let components = line.split(" ").collect::<Vec<_>>();
        match components.len() {
            0 => {
                iter.next();
            }
            1 => {
                // Handle as a directory
                let dir = root.join(&components[0]);
                std::fs::create_dir(&dir).ok();
                iter.next();
                let mut combined = String::new();
                while let Some(line) = iter.peek() {
                    if line.starts_with("- ") {
                        combined.push_str(&line[2..]);
                        combined.push('\n');
                        iter.next();
                    } else {
                        break;
                    }
                }
                write_files(&dir, &combined).unwrap();
            }
            2 => {
                // Handle as a file
                let p = root.join(components[0]);
                std::fs::write(p, components[1]).unwrap();
                iter.next();
            }
            _ => {
                return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
            }
        }
    }
    Ok(())
}

fn setup_repository() {
    let data_dir = std::path::Path::new(DATA_DIR);
    std::fs::create_dir_all("/tmp/src_integration/spaces/z01").unwrap();
    std::env::set_current_dir("/tmp/src_integration/spaces/z01").unwrap();
    cli::create(
        data_dir.to_owned(),
        "localhost:44959/colin/test".to_string(),
    );
    cli::checkout(
        data_dir.to_owned(),
        String::new(),
        "localhost:44959/colin/test".to_string(),
    );
    write_files(
        std::path::Path::new("/tmp/src_integration/spaces/z01"),
        "
        README.txt hello_world
        another_file content
        dir
         - zombo.com 101010
    ",
    )
    .unwrap();
    cli::push(data_dir.to_owned(), "initial code".to_string());

    // Should be submitted as localhost:44959/colin/test/2
    cli::submit(data_dir.to_owned());
}

fn setup_repository_3() {
    let data_dir = std::path::Path::new(DATA_DIR);
    std::fs::create_dir_all("/tmp/src_integration/spaces/x01").unwrap();
    std::env::set_current_dir("/tmp/src_integration/spaces/x01").unwrap();
    cli::create(
        data_dir.to_owned(),
        "localhost:44959/colin/program".to_string(),
    );
    cli::checkout(
        data_dir.to_owned(),
        String::new(),
        "localhost:44959/colin/program".to_string(),
    );
    std::fs::write(
        "/tmp/src_integration/spaces/x01/main.rs",
        r#"
fn main() {
    // Some content
    println!("hello, world!");
}
"#,
    )
    .unwrap();
    cli::push(data_dir.to_owned(), "initial code".to_string());

    // Should be submitted as localhost:44959/colin/example/2
    cli::submit(data_dir.to_owned());
}

fn setup_repository_2() {
    let data_dir = std::path::Path::new(DATA_DIR);
    std::fs::create_dir_all("/tmp/src_integration/spaces/y01").unwrap();
    std::env::set_current_dir("/tmp/src_integration/spaces/y01").unwrap();
    cli::create(
        data_dir.to_owned(),
        "localhost:44959/colin/example".to_string(),
    );
    cli::checkout(
        data_dir.to_owned(),
        String::new(),
        "localhost:44959/colin/example".to_string(),
    );
    write_files(
        std::path::Path::new("/tmp/src_integration/spaces/y01"),
        "
        main.cc int_main(;;)
        Makefile zzzz101010
    ",
    )
    .unwrap();
    cli::push(data_dir.to_owned(), "initial code".to_string());

    // Should be submitted as localhost:44959/colin/example/2
    cli::submit(data_dir.to_owned());
}

async fn run_test(
    name: &'static str,
    f: impl FnOnce() + Sync + Send + 'static,
    filters: &[String],
) -> Result<(), ()> {
    for filter in filters {
        if !name.contains(filter) {
            return Ok(());
        }
    }

    let term = tui::Terminal::new();
    term.set_underline();
    print!("RUN\t");
    term.set_normal();
    println!("{}", name);

    if in_cleanroom(f).await {
        term.set_underline();
        print!("PASSED\t");
        term.set_normal();
        println!("{}", name);
        Ok(())
    } else {
        term.set_underline();
        print!("FAILED\t");
        term.set_normal();
        println!("{}", name);
        Err(())
    }
}

#[tokio::main]
async fn main() {
    let args = flags::parse_flags!();

    run_test(
        "submit and checkout",
        || {
            let data_dir = std::path::Path::new(DATA_DIR);
            setup_repository();

            // We've submitted a change in the previous step, so the repo should be at 2
            let d = src_lib::Src::new(data_dir.to_owned()).expect("failed to initialize src!");
            let client = d.get_client("localhost:44959").unwrap();
            let repo = match client.get_repository(service::GetRepositoryRequest {
                token: String::new(),
                owner: "colin".to_string(),
                name: "test".to_string(),
                ..Default::default()
            }) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("failed to checkout: {:?}", e);
                    std::process::exit(1);
                }
            };
            assert_eq!(repo.index, 2);

            std::fs::create_dir_all("/tmp/src_integration/spaces/z02").unwrap();
            std::env::set_current_dir("/tmp/src_integration/spaces/z02").unwrap();
            cli::checkout(
                data_dir.to_owned(),
                "my-alias".to_string(),
                "localhost:44959/colin/test".to_string(),
            );
            assert_eq!(
                &std::fs::read_to_string("/tmp/src_integration/spaces/z02/another_file").unwrap(),
                "content"
            );

            // Now create some further changes on top of that...
            write_files(
                std::path::Path::new("/tmp/src_integration/spaces/z02"),
                "
        another_file content2
        dir
         - newfile 1001001
    ",
            )
            .unwrap();
            cli::snapshot(data_dir.to_owned(), "some updates".to_string());

            let s = d.get_latest_snapshot("my-alias").unwrap().unwrap();
            assert_eq!(s.files.len(), 2);
            assert_eq!(s.files[0].path, "another_file");
            assert_eq!(s.files[0].kind, service::DiffKind::Modified);
            assert_eq!(s.files[1].path, "dir/newfile");
            assert_eq!(s.files[1].kind, service::DiffKind::Added);
            assert_eq!(&s.message, "some updates");

            // Delete the directory and observe the diff
            std::fs::remove_dir_all("/tmp/src_integration/spaces/z02/dir").unwrap();

            // Diff should see new files
            let resp = d
                .diff(service::DiffRequest {
                    dir: "/tmp/src_integration/spaces/z02".to_string(),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(resp.failed, false);
            assert_eq!(
                resp.files.iter().map(|f| &f.path).collect::<Vec<_>>(),
                vec!["another_file", "dir", "dir/zombo.com"]
            );
        },
        &args,
    )
    .await
    .unwrap();

    run_test(
        "create, delete, revert",
        || {
            let data_dir = std::path::Path::new(DATA_DIR);
            setup_repository();

            std::fs::create_dir_all("/tmp/src_integration/spaces/z02").unwrap();
            std::env::set_current_dir("/tmp/src_integration/spaces/z02").unwrap();
            cli::checkout(
                data_dir.to_owned(),
                "my-alias".to_string(),
                "localhost:44959/colin/test".to_string(),
            );

            // Make sure checkout worked OK
            assert_eq!(
                &std::fs::read_to_string("/tmp/src_integration/spaces/z02/another_file").unwrap(),
                "content"
            );

            // Diff should have no changes
            let d = src_lib::Src::new(data_dir.to_owned()).expect("failed to initialize src!");
            let resp = d
                .diff(service::DiffRequest {
                    dir: "/tmp/src_integration/spaces/z02".to_string(),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(resp.failed, false);
            assert_eq!(resp.files.len(), 0);

            // Now create some further changes on top of that...
            write_files(
                std::path::Path::new("/tmp/src_integration/spaces/z02"),
                "
        another_file content2
        dir
         - newfile 1001001
    ",
            )
            .unwrap();

            // Diff should see new files
            let d = src_lib::Src::new(data_dir.to_owned()).expect("failed to initialize src!");
            let resp = d
                .diff(service::DiffRequest {
                    dir: "/tmp/src_integration/spaces/z02".to_string(),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(resp.failed, false);
            assert_eq!(
                resp.files.iter().map(|f| &f.path).collect::<Vec<_>>(),
                vec!["another_file", "dir/newfile",]
            );

            // Revert changes
            cli::revert(
                data_dir.to_owned(),
                &["another_file".to_string(), "dir/newfile".to_string()],
            );

            // No changes after revert
            let d = src_lib::Src::new(data_dir.to_owned()).expect("failed to initialize src!");
            let resp = d
                .diff(service::DiffRequest {
                    dir: "/tmp/src_integration/spaces/z02".to_string(),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(resp.failed, false);
            assert_eq!(resp.files.len(), 0);

            // Delete some files
            std::fs::remove_file("another_file").unwrap();

            // Deleted file appears in diff as removed
            let d = src_lib::Src::new(data_dir.to_owned()).expect("failed to initialize src!");
            let resp = d
                .diff(service::DiffRequest {
                    dir: "/tmp/src_integration/spaces/z02".to_string(),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(resp.failed, false);
            assert_eq!(resp.files.len(), 1);
            assert_eq!(resp.files[0].path, "another_file");
            assert_eq!(resp.files[0].kind, service::DiffKind::Removed);

            // Revert deletion
            cli::revert(data_dir.to_owned(), &["another_file".to_string()]);

            // File should be back after revert
            assert!(std::path::Path::new("/tmp/src_integration/spaces/z02/another_file").exists());
        },
        &args,
    )
    .await
    .unwrap();

    run_test(
        "attach and detach spaces",
        || {
            let data_dir = std::path::Path::new(DATA_DIR);
            setup_repository();
            setup_repository_2();

            std::fs::create_dir_all("/tmp/src_integration/spaces/z02").unwrap();
            std::env::set_current_dir("/tmp/src_integration/spaces/z02").unwrap();
            cli::checkout(
                data_dir.to_owned(),
                "my-alias".to_string(),
                "localhost:44959/colin/test".to_string(),
            );
            assert_eq!(
                &std::fs::read_to_string("/tmp/src_integration/spaces/z02/another_file").unwrap(),
                "content"
            );

            // Write some more changes to the space before checking out something else
            write_files(
                std::path::Path::new("/tmp/src_integration/spaces/z02"),
                "
        another_file content2
        dir
         - newfile 1001001
    ",
            )
            .unwrap();

            // Checkout another repository. This should snapshot the unsaved changes to my-alias
            cli::checkout(
                data_dir.to_owned(),
                "other-project".to_string(),
                "localhost:44959/colin/example".to_string(),
            );
            assert!(std::path::Path::new("/tmp/src_integration/spaces/z02/main.cc").exists());
            assert!(!std::path::Path::new("/tmp/src_integration/spaces/z02/another_file").exists());
            assert!(!std::path::Path::new("/tmp/src_integration/spaces/z02/dir/newfile").exists());
            assert!(!std::path::Path::new("/tmp/src_integration/spaces/z02/dir").exists());

            // Check out the original one again, this time it should restore the snapshotted changes as well
            cli::checkout(data_dir.to_owned(), String::new(), "my-alias".to_string());
            assert!(!std::path::Path::new("/tmp/src_integration/spaces/z02/main.cc").exists());
            assert!(std::path::Path::new("/tmp/src_integration/spaces/z02/another_file").exists());
            assert_eq!(
                &std::fs::read_to_string("/tmp/src_integration/spaces/z02/another_file").unwrap(),
                "content2"
            );
            assert!(std::path::Path::new("/tmp/src_integration/spaces/z02/dir/newfile").exists());
            assert!(std::path::Path::new("/tmp/src_integration/spaces/z02/dir").exists());
        },
        &args,
    )
    .await
    .unwrap();

    run_test(
        "sync",
        || {
            let data_dir = std::path::Path::new(DATA_DIR);
            setup_repository();

            // Check out the repository and don't modify it yet
            std::fs::create_dir_all("/tmp/src_integration/spaces/z03").unwrap();
            std::env::set_current_dir("/tmp/src_integration/spaces/z03").unwrap();
            cli::checkout(
                data_dir.to_owned(),
                "laggard".to_string(),
                "localhost:44959/colin/test".to_string(),
            );

            write_files(
                std::path::Path::new("/tmp/src_integration/spaces/z03"),
                "
        newf newcontent
    ",
            )
            .unwrap();

            // Check out in another space, modify and submit
            std::fs::create_dir_all("/tmp/src_integration/spaces/z02").unwrap();
            std::env::set_current_dir("/tmp/src_integration/spaces/z02").unwrap();
            cli::checkout(
                data_dir.to_owned(),
                "my-alias".to_string(),
                "localhost:44959/colin/test".to_string(),
            );
            write_files(
                std::path::Path::new("/tmp/src_integration/spaces/z02"),
                "
        another_file content2
        dir
         - newfile 1001001
    ",
            )
            .unwrap();
            cli::push(data_dir.to_owned(), "small changes".to_string());
            cli::submit(data_dir.to_owned());

            // Should be submitted as localhost:44959/colin/example/3. Now go back to the old space
            // and sync.
            std::env::set_current_dir("/tmp/src_integration/spaces/z03").unwrap();
            cli::sync(data_dir.to_owned(), std::collections::HashMap::new());

            // Should observe modifications to `another_file` and `dir/newfile`
            assert_eq!(
                &std::fs::read_to_string("/tmp/src_integration/spaces/z03/another_file").unwrap(),
                "content2"
            );
            assert_eq!(
                &std::fs::read_to_string("/tmp/src_integration/spaces/z03/dir/newfile").unwrap(),
                "1001001"
            );
        },
        &args,
    )
    .await
    .unwrap();

    run_test(
        "sync with conflicts",
        || {
            let data_dir = std::path::Path::new(DATA_DIR);
            setup_repository_3();

            // Check out the repository and change it
            std::fs::create_dir_all("/tmp/src_integration/spaces/z03").unwrap();
            std::env::set_current_dir("/tmp/src_integration/spaces/z03").unwrap();
            cli::checkout(
                data_dir.to_owned(),
                "version_a".to_string(),
                "localhost:44959/colin/program".to_string(),
            );

            std::fs::write(
                "/tmp/src_integration/spaces/z03/main.rs",
                r#"
fn main() {
    // Some content that I changed
    println!("hello, world!");
}
"#,
            )
            .unwrap();

            // Check out in another space, modify and submit
            std::fs::create_dir_all("/tmp/src_integration/spaces/z02").unwrap();
            std::env::set_current_dir("/tmp/src_integration/spaces/z02").unwrap();
            cli::checkout(
                data_dir.to_owned(),
                "version_b".to_string(),
                "localhost:44959/colin/program".to_string(),
            );

            std::fs::write(
                "/tmp/src_integration/spaces/z02/main.rs",
                r#"
fn main() {
    // Some content which I have modified
    println!("hello, world!");
}
"#,
            )
            .unwrap();
            cli::push(data_dir.to_owned(), "small changes".to_string());
            cli::submit(data_dir.to_owned());

            // Should be submitted as localhost:44959/colin/program/3. Now go back to the old space
            // and sync.
            std::env::set_current_dir("/tmp/src_integration/spaces/z03").unwrap();

            let mut resolutions = std::collections::HashMap::new();
            let data = "my resolution".as_bytes().to_owned();
            resolutions.insert(
                "main.rs".to_string(),
                core::ConflictResolutionOverride::Merged(data),
            );

            cli::sync(data_dir.to_owned(), resolutions);

            assert_eq!(
                &std::fs::read_to_string("/tmp/src_integration/spaces/z03/main.rs").unwrap(),
                "my resolution"
            );
        },
        &args,
    )
    .await
    .unwrap();
}

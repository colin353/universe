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

    // Should be submitted as localhost:44959/colin/test
    cli::submit(data_dir.to_owned());
}

async fn run_test(name: &'static str, f: impl FnOnce() + Sync + Send + 'static) {
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
    } else {
        term.set_underline();
        print!("FAILED\t");
        term.set_normal();
        println!("{}", name);
    }
}

#[tokio::main]
async fn main() {
    run_test("submit and checkout", || {
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
            String::new(),
            "localhost:44959/colin/test".to_string(),
        );
        assert_eq!(
            &std::fs::read_to_string("/tmp/src_integration/spaces/z02/another_file").unwrap(),
            "content"
        );
    })
    .await;
}

const DATA_DIR: &'static str = "/tmp/src_integration/src";

async fn in_cleanroom(f: impl FnOnce() + Sync + Send + 'static) {
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
        }
        _ = rx => {
            println!("tests finished");
            return
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

#[tokio::main]
async fn main() {
    in_cleanroom(|| setup_repository()).await
}

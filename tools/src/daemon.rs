use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::Read;

#[derive(Debug)]
struct FileState {
    mtime: u64,
    length: u64,
    hash: [u8; 32],
}

fn diff(
    path: &std::path::Path,
    state: &HashMap<std::path::PathBuf, FileState>,
) -> std::io::Result<service::DiffResponse> {
    let mut expected: HashMap<&std::path::Path, Vec<&std::path::Path>> = HashMap::new();
    for (p, _) in state {
        if p == path {
            continue;
        }

        let parent = match p.parent() {
            Some(p) => p,
            None => continue,
        };
        expected.entry(parent).or_insert(Vec::new()).push(p);
    }

    for (_, v) in expected.iter_mut() {
        v.sort();
    }

    let mut resp = service::DiffResponse::new();
    diff_from(path, path, state, &expected, &mut resp.files);
    Ok(resp)
}

fn mtime(m: &std::fs::Metadata) -> u64 {
    let mt = match m.modified() {
        Ok(m) => m,
        Err(_) => return 0,
    };
    let since_epoch = mt.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

fn metadata_compatible(state: &FileState, m: &std::fs::Metadata) -> bool {
    if state.length != m.len() {
        return false;
    }

    if mtime(m) == state.mtime {
        return true;
    }
    false
}

fn hash_file(path: &std::path::Path) -> std::io::Result<[u8; 32]> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0_u8; 8192];
    let mut h = DefaultHasher::new();

    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.write(&buf);
    }

    let bytes = h.finish().to_le_bytes();
    Ok([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ])
}

fn diff_from(
    root: &std::path::Path,
    path: &std::path::Path,
    state: &HashMap<std::path::PathBuf, FileState>,
    expected: &HashMap<&std::path::Path, Vec<&std::path::Path>>,
    differences: &mut Vec<service::FileDiff>,
) -> std::io::Result<()> {
    let mut observed_paths = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if ty.is_symlink() {
            continue;
        }

        let path = entry.path();
        observed_paths.push(path.clone());

        // Entry is a directory. Only recurse if the mtime has changed.
        if ty.is_dir() {
            let mut should_recurse = true;

            if let Some(s) = state.get(&path) {
                let metadata = entry.metadata()?;
                if metadata_compatible(s, &metadata) {
                    should_recurse = false;
                }
            } else {
                differences.push(service::FileDiff {
                    path: path
                        .strip_prefix(root)
                        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_str()
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_string(),
                    differences: vec![],
                    is_dir: true,
                    kind: service::DiffKind::Added,
                });
            }
            if should_recurse {
                diff_from(root, &path, state, expected, differences);
            }

            continue;
        }

        // Entry is a file.
        if let Some(s) = state.get(&path) {
            let metadata = entry.metadata()?;
            if metadata_compatible(s, &metadata) {
                continue;
            }

            if hash_file(&path)? == s.hash {
                continue;
            }

            differences.push(service::FileDiff {
                path: path
                    .strip_prefix(root)
                    .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                    .to_str()
                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                    .to_string(),
                differences: vec![],
                is_dir: false,
                kind: service::DiffKind::Modified,
            });
        } else {
            differences.push(service::FileDiff {
                path: path
                    .strip_prefix(root)
                    .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                    .to_str()
                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                    .to_string(),
                differences: vec![],
                is_dir: false,
                kind: service::DiffKind::Added,
            });
        }
    }

    observed_paths.sort();

    let nothing = Vec::new();
    let mut observed_iter = observed_paths.iter().peekable();
    let mut expected_iter = expected.get(path).unwrap_or(&nothing).iter().peekable();

    loop {
        match (expected_iter.peek(), observed_iter.peek()) {
            (Some(exp), Some(obs)) => {
                if exp == obs {
                    expected_iter.next();
                    observed_iter.next();
                    continue;
                }

                if obs > exp {
                    // We missed an expected document. Report it as missing
                    differences.push(service::FileDiff {
                        path: exp
                            .strip_prefix(root)
                            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                            .to_str()
                            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                            .to_string(),
                        differences: vec![],
                        is_dir: false,
                        kind: service::DiffKind::Removed,
                    });

                    expected_iter.next();
                } else {
                    // We got an extra document, this case is already covered
                    observed_iter.next();
                }
            }
            (Some(exp), None) => {
                // We missed an expected document. Report it as missing
                differences.push(service::FileDiff {
                    path: exp
                        .strip_prefix(root)
                        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_str()
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_string(),
                    differences: vec![],
                    is_dir: false,
                    kind: service::DiffKind::Removed,
                });

                expected_iter.next();
            }
            _ => break,
        }
    }

    Ok(())
}

fn snapshot_state(
    path: &std::path::Path,
) -> std::io::Result<HashMap<std::path::PathBuf, FileState>> {
    let mut out = HashMap::new();
    snapshot_state_from(path, &mut out)?;
    Ok(out)
}

fn snapshot_state_from(
    path: &std::path::Path,
    out: &mut HashMap<std::path::PathBuf, FileState>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            continue;
        }

        let metadata = entry.metadata()?;
        if ty.is_dir() {
            out.insert(
                entry.path().to_owned(),
                FileState {
                    mtime: mtime(&metadata),
                    length: 0,
                    hash: [0; 32],
                },
            );

            snapshot_state_from(&entry.path(), out);
        } else {
            out.insert(
                entry.path().to_owned(),
                FileState {
                    mtime: mtime(&metadata),
                    length: metadata.len(),
                    hash: hash_file(&entry.path())?,
                },
            );
        }
    }

    Ok(())
}

fn main() {
    let dir = std::path::Path::new("/tmp/tree");
    let state = snapshot_state(dir).unwrap();
    println!("snapshot: {:#?}", state);
    std::io::stdin().read_line(&mut String::new()).unwrap();
    println!("{:#?}", diff(dir, &state).unwrap());
}

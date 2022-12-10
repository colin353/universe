use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bus::{Deserialize, Serialize};

mod helpers;
mod metadata;

pub struct Src {
    // table: managed_largetable::ManagedLargeTable,
    root: std::path::PathBuf,
    remotes: RwLock<HashMap<String, service::SrcServerClient>>,
}

impl Src {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
        Self::init(&root)?;

        Ok(Self {
            root,
            remotes: RwLock::new(HashMap::new()),
        })
    }

    pub fn get_client(&self, host: &str) -> std::io::Result<service::SrcServerClient> {
        // Check if the client already exists
        match self
            .remotes
            .read()
            .expect("failed to read lock remotes")
            .get(host)
        {
            Some(client) => return Ok(client.clone()),
            None => (),
        }

        // Client doesn't exist, create it
        let (hostname, port): (&str, u16) = match host.find(":") {
            Some(idx) => {
                let port = match host[idx + 1..].parse::<u16>() {
                    Ok(p) => p,
                    Err(_) => {
                        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
                    }
                };

                (&host[0..idx], port)
            }
            None => (&host, 4959),
        };

        let connector = Arc::new(bus_rpc::HyperSyncClient::new(hostname.to_string(), port));
        let client = service::SrcServerClient::new(connector);

        self.remotes
            .write()
            .expect("failed to write lock remotes")
            .insert(hostname.to_string(), client.clone());

        Ok(client)
    }

    pub fn get_metadata(&self, basis: service::BasisView) -> std::io::Result<metadata::Metadata> {
        // Check whether we already downloaded the metadata
        let metadata_path = self
            .root
            .join("metadata")
            .join(basis.get_host())
            .join(basis.get_owner())
            .join(basis.get_name())
            .with_extension("sstable");

        if metadata_path.exists() {
            return metadata::Metadata::from_path(&metadata_path);
        }

        let client = self.get_client(basis.get_host())?;
        let resp = client
            .get_metadata(service::GetMetadataRequest {
                token: String::new(),
                basis: basis.to_owned()?,
            })
            .map_err(|e| {
                eprintln!("{:?}", e);
                std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "failed to connect to host",
                )
            })?;

        if resp.failed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("failed to download metadata: {}", resp.error_message),
            ));
        }

        if let Some(p) = metadata_path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(&metadata_path, &resp.data)?;

        metadata::Metadata::from_path(&metadata_path)
    }

    fn diff_from(
        &self,
        root: &std::path::Path,
        path: &std::path::Path,
        metadata: &metadata::Metadata,
        differences: &mut Vec<service::FileDiff>,
    ) -> std::io::Result<()> {
        let get_metadata = |p: &std::path::Path| -> Option<service::FileView> {
            let path_str = p.to_str()?;
            let key = format!("{}/{}", path_str.split("/").count(), path_str);
            metadata.get(&key)
        };

        let mut observed_paths = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let ty = entry.file_type()?;

            if ty.is_symlink() {
                continue;
            }

            let path = entry.path();
            observed_paths.push(path.clone());

            let relative_path = path
                .strip_prefix(root)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;

            // Entry is a directory. Only recurse if the mtime has changed.
            if ty.is_dir() {
                let mut should_recurse = true;

                if let Some(s) = get_metadata(&relative_path) {
                    let metadata = entry.metadata()?;
                    if helpers::metadata_compatible(s, &metadata) {
                        should_recurse = false;
                    }
                } else {
                    differences.push(service::FileDiff {
                        path: relative_path
                            .to_str()
                            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                            .to_string(),
                        differences: vec![],
                        is_dir: true,
                        kind: service::DiffKind::Added,
                    });
                }
                if should_recurse {
                    self.diff_from(root, &path, metadata, differences)?;
                }

                continue;
            }

            // Entry is a file.
            if let Some(s) = get_metadata(&relative_path) {
                let metadata = entry.metadata()?;
                if helpers::metadata_compatible(s, &metadata) {
                    continue;
                }

                let modified = std::fs::read(&path)?;
                if core::hash_bytes(&modified) == s.get_sha() {
                    continue;
                }

                let original = match self.get_blob(s.get_sha()) {
                    Some(o) => o,
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("blob {:?} not found!", core::fmt_sha(s.get_sha())),
                        ));
                    }
                };

                differences.push(service::FileDiff {
                    path: relative_path
                        .to_str()
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_string(),
                    differences: core::diff(&original, &modified),
                    is_dir: false,
                    kind: service::DiffKind::Modified,
                });
            } else {
                let mut data = Vec::new();
                core::compress_rw(
                    &mut std::io::BufReader::new(std::fs::File::open(&path)?),
                    &mut data,
                )?;

                differences.push(service::FileDiff {
                    path: relative_path
                        .to_str()
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_string(),
                    differences: vec![service::ByteDiff {
                        start: 0,
                        end: 0,
                        kind: service::DiffKind::Added,
                        data,
                        compression: service::CompressionKind::LZ4,
                    }],
                    is_dir: false,
                    kind: service::DiffKind::Added,
                });
            }
        }

        observed_paths.sort();

        let nothing: Vec<std::path::PathBuf> = Vec::new();
        let mut observed_iter = observed_paths.iter().peekable();
        let mut expected_iter = nothing.iter().peekable();

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
                                .map_err(|_| {
                                    std::io::Error::from(std::io::ErrorKind::InvalidInput)
                                })?
                                .to_str()
                                .ok_or_else(|| {
                                    std::io::Error::from(std::io::ErrorKind::InvalidInput)
                                })?
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

    pub fn initialize_repo(
        &self,
        basis: service::Basis,
        path: &std::path::Path,
    ) -> std::io::Result<String> {
        let alias = self.find_unused_alias(&basis.name);

        std::fs::create_dir_all(self.get_change_path(&alias)).ok();
        let f = std::fs::File::create(self.get_change_metadata_path(&alias))?;
        let change = service::Space {
            basis,
            change_id: 0,
            directory: path
                .to_str()
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "path is not valid utf-8!",
                    )
                })?
                .to_string(),
        };
        change.encode(&mut std::io::BufWriter::new(f))?;
        std::fs::write(self.get_change_dir_path(path), alias.as_bytes())?;

        Ok(alias)
    }

    pub fn diff(&self, req: service::DiffRequest) -> std::io::Result<service::DiffResponse> {
        let change = if !req.dir.is_empty() {
            match self.get_change_by_dir(std::path::Path::new(&req.dir)) {
                Some(c) => c,
                _ => {
                    return Ok(service::DiffResponse {
                        failed: true,
                        error_message: format!("{} is not a src directory!", req.dir),
                        ..Default::default()
                    })
                }
            }
        } else {
            match self.get_change_by_alias(&req.alias) {
                Some(c) => c,
                _ => {
                    return Ok(service::DiffResponse {
                        failed: true,
                        error_message: format!("unrecognized change {}", req.alias),
                        ..Default::default()
                    })
                }
            }
        };

        let basis = change.basis;
        let directory = std::path::PathBuf::from(change.directory);

        let metadata = self.get_metadata(basis.as_view())?;
        let mut differences = Vec::new();

        self.diff_from(&directory, &directory, &metadata, &mut differences)?;
        differences.sort_by_cached_key(|d| d.path.clone());

        Ok(service::DiffResponse {
            files: differences,
            basis,
            ..Default::default()
        })
    }

    pub fn snapshot(
        &self,
        req: service::SnapshotRequest,
    ) -> std::io::Result<service::SnapshotResponse> {
        let alias = if req.alias.is_empty() {
            match self.get_change_alias_by_dir(&std::path::Path::new(&req.dir)) {
                Some(a) => a,
                _ => {
                    return Ok(service::SnapshotResponse {
                        failed: true,
                        error_message: format!("{} is not a src directory!", req.dir),
                        ..Default::default()
                    })
                }
            }
        } else {
            req.alias
        };

        let diff = self.diff(service::DiffRequest {
            alias: alias.clone(),
            ..Default::default()
        })?;

        if diff.failed {
            return Ok(service::SnapshotResponse {
                failed: true,
                error_message: format!("failed to diff: {}", diff.error_message),
                ..Default::default()
            });
        }

        let ts = core::timestamp_usec();
        let snapshot = service::Snapshot {
            timestamp: ts,
            basis: diff.basis,
            files: diff.files,
            message: req.message,
        };

        let f = std::fs::File::create(self.get_snapshot_path(&alias, ts))?;
        snapshot.encode(&mut std::io::BufWriter::new(f))?;
        Ok(service::SnapshotResponse {
            timestamp: ts,
            ..Default::default()
        })
    }

    pub fn new_space(
        &self,
        req: service::NewSpaceRequest,
    ) -> std::io::Result<service::NewSpaceResponse> {
        let directory = std::path::PathBuf::from(&req.dir);
        let directory_str = req.dir;

        // Check that the directory is empty.
        if directory
            .read_dir()
            .map(|mut i| i.next().is_none())
            .unwrap_or(false)
        {
            return Ok(service::NewSpaceResponse {
                failed: true,
                error_message: String::from("change directory must be empty!"),
                ..Default::default()
            });
        }

        let index = self.validate_basis(req.basis.as_view())?;
        std::fs::create_dir_all(self.get_change_path(&req.alias)).ok();
        let f = std::fs::File::create(self.get_change_metadata_path(&req.alias))?;

        let space = service::Space {
            change_id: 0,
            basis: service::Basis {
                host: req.basis.host.clone(),
                owner: req.basis.owner.clone(),
                name: req.basis.name.clone(),
                change: 0,
                index,
            },
            directory: directory_str.clone(),
        };
        space.encode(&mut std::io::BufWriter::new(f))?;

        std::fs::write(
            self.get_change_dir_path(&std::path::Path::new(&directory_str)),
            req.alias.as_bytes(),
        )?;

        let metadata = self.get_metadata(
            service::Basis {
                host: req.basis.host.clone(),
                owner: req.basis.owner.clone(),
                name: req.basis.name.clone(),
                change: 0,
                index,
            }
            .as_view(),
        )?;

        let client = self.get_client(&req.basis.host)?;

        let mut to_download: HashMap<Vec<u8>, Vec<(String, service::FileView)>> = HashMap::new();
        let mut directories: HashMap<usize, Vec<(std::path::PathBuf, service::FileView)>> =
            HashMap::new();
        for (key, file) in metadata.iter() {
            let (depth, path) = match key.find('/') {
                Some(idx) => {
                    let depth = key[0..idx].parse::<usize>().map_err(|_| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "metadata path did not contain numeric leading depth!",
                        )
                    })?;
                    (depth, key[idx + 1..].to_string())
                }
                None => {
                    return Ok(service::NewSpaceResponse {
                        failed: true,
                        error_message: "invalid metadata path!".to_string(),
                        ..Default::default()
                    });
                }
            };

            if file.get_is_dir() {
                directories
                    .entry(depth)
                    .or_insert_with(Vec::new)
                    .push((directory.join(path), file));
                continue;
            }

            match self.get_blob(file.get_sha()) {
                Some(b) => self.write_file(&directory.join(path), file, &b)?,
                None => {
                    to_download
                        .entry(file.get_sha().to_owned())
                        .or_insert_with(Vec::new)
                        .push((path, file));
                }
            }

            if to_download.len() >= 250 {
                let resp = match client.get_blobs(service::GetBlobsRequest {
                    token: String::new(),
                    shas: to_download.iter().map(|(sha, _)| sha.to_vec()).collect(),
                }) {
                    Err(_) => {
                        return Ok(service::NewSpaceResponse {
                            failed: true,
                            error_message: "failed to download blobs!".to_string(),
                            ..Default::default()
                        });
                    }
                    Ok(r) => r,
                };

                if resp.failed {
                    return Ok(service::NewSpaceResponse {
                        failed: true,
                        error_message: "failed to download blobs!".to_string(),
                        ..Default::default()
                    });
                }

                if resp.blobs.len() != to_download.len() {
                    return Ok(service::NewSpaceResponse {
                        failed: true,
                        error_message: "failed to download all blobs!".to_string(),
                        ..Default::default()
                    });
                }

                for blob in &resp.blobs {
                    let files = match to_download.get(&blob.sha) {
                        Some(p) => p,
                        None => {
                            return Ok(service::NewSpaceResponse {
                                failed: true,
                                error_message: "got an unexpected blob".to_string(),
                                ..Default::default()
                            });
                        }
                    };

                    for (path, file) in files {
                        self.write_file(&directory.join(path), *file, &blob.data)?;
                    }

                    std::fs::write(self.get_blob_path(&blob.sha), &blob.data)?;
                }

                to_download.clear();
            }
        }

        if to_download.len() > 0 {
            let resp = match client.get_blobs(service::GetBlobsRequest {
                token: String::new(),
                shas: to_download.iter().map(|(sha, _)| sha.to_vec()).collect(),
            }) {
                Ok(r) => r,
                Err(_) => {
                    return Ok(service::NewSpaceResponse {
                        failed: true,
                        error_message: "failed to download blobs!".to_string(),
                        ..Default::default()
                    });
                }
            };

            if resp.failed {
                return Ok(service::NewSpaceResponse {
                    failed: true,
                    error_message: "failed to download blobs!".to_string(),
                    ..Default::default()
                });
            }

            if resp.blobs.len() != to_download.len() {
                return Ok(service::NewSpaceResponse {
                    failed: true,
                    error_message: "failed to download all blobs!".to_string(),
                    ..Default::default()
                });
            }

            for blob in &resp.blobs {
                let files = match to_download.get(&blob.sha) {
                    Some(p) => p,
                    None => {
                        return Ok(service::NewSpaceResponse {
                            failed: true,
                            error_message: "got an unexpected blob".to_string(),
                            ..Default::default()
                        });
                    }
                };

                for (path, file) in files {
                    self.write_file(&directory.join(path), *file, &blob.data)?;
                }

                std::fs::write(self.get_blob_path(&blob.sha), &blob.data)?;
            }
        }

        let mut dirs: Vec<(usize, Vec<(std::path::PathBuf, service::FileView)>)> =
            directories.into_iter().collect();
        dirs.sort_by_key(|(depth, _)| *depth);
        for (_, items) in dirs.into_iter().rev() {
            for (path, file) in items {
                self.write_dir(&directory.join(path), file)?;
            }
        }

        Ok(service::NewSpaceResponse {
            dir: directory_str,
            index,
            ..Default::default()
        })
    }

    pub fn list_changes(&self) -> std::io::Result<Vec<(String, service::Space)>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.root.join("changes").join("by_alias"))? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(f) = entry.file_name().to_str() {
                    if let Some(c) = self.get_change_by_alias(f) {
                        out.push((f.to_string(), c));
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn list_snapshots(&self, alias: &str) -> std::io::Result<Vec<service::Snapshot>> {
        let mut snapshots = Vec::new();
        for entry in std::fs::read_dir(self.get_change_path(alias))? {
            let path = entry?.path();
            if let Some("snapshot") = path.extension().map(|s| s.to_str()).flatten() {
                let bytes = std::fs::read(path)?;
                snapshots.push(service::Snapshot::decode(&bytes)?);
            }
        }
        snapshots.sort_by_key(|c| std::cmp::Reverse(c.timestamp));
        Ok(snapshots)
    }

    pub fn get_latest_snapshot(&self, alias: &str) -> std::io::Result<Option<service::Snapshot>> {
        let mut candidate: Option<service::Snapshot> = None;
        for entry in std::fs::read_dir(self.get_change_path(alias))? {
            let path = entry?.path();
            if let Some("snapshot") = path.extension().map(|s| s.to_str()).flatten() {
                let bytes = std::fs::read(path)?;
                let s = service::Snapshot::decode(&bytes)?;
                if let Some(c) = candidate.as_ref() {
                    if c.timestamp < s.timestamp {
                        candidate = Some(s);
                    }
                } else {
                    candidate = Some(s);
                }
            }
        }
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use service::*;

    fn server() -> service::SrcServerClient {
        let connector = Arc::new(bus_rpc::HyperClient::new(String::from("127.0.0.1"), 4959));
        service::SrcServerClient::new(connector)
    }

    fn src() -> Src {
        Src::new(std::path::PathBuf::from("/tmp/code")).unwrap()
    }

    fn test_checkout() {
        let d = src();
        let resp = d
            .new_space(NewSpaceRequest {
                dir: "/tmp/code/my-branch".to_string(),
                alias: "my-branch".to_string(),
                basis: Basis {
                    host: "127.0.0.1".to_string(),
                    owner: "colin".to_string(),
                    name: "example".to_string(),
                    change: 0,
                    index: 0,
                },
            })
            .unwrap();

        println!("{:?}", resp);
        assert_eq!(resp.failed, false);
    }

    #[test]
    fn test_diff() {
        let d = src();
        let resp = d
            .diff(DiffRequest {
                alias: "my-branch".to_string(),
                ..Default::default()
            })
            .unwrap();
        println!("{:#?}", resp);
        assert_eq!(resp.failed, false);
    }

    fn test_system() {
        let s = server();

        let resp = s
            .create(CreateRequest {
                token: String::new(),
                name: "example".to_string(),
            })
            .unwrap();
        assert_eq!(resp.failed, false);

        s.submit(SubmitRequest {
            token: String::new(),
            basis: Basis {
                owner: "colin".to_string(),
                name: "example".to_string(),
                index: 1,
                ..Default::default()
            },
            files: vec![
                FileDiff {
                    path: "a.txt".to_string(),
                    kind: DiffKind::Added,
                    is_dir: false,
                    differences: vec![ByteDiff {
                        data: "hello world\n".to_string().into_bytes(),
                        ..Default::default()
                    }],
                },
                FileDiff {
                    path: "dir/b.txt".to_string(),
                    kind: DiffKind::Added,
                    is_dir: false,
                    differences: vec![ByteDiff {
                        data: "change da world\n".to_string().into_bytes(),
                        ..Default::default()
                    }],
                },
            ],
        })
        .unwrap();
    }
}

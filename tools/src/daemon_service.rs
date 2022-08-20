use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;

use bus::{Deserialize, Serialize};

#[derive(Debug)]
struct FileState {
    mtime: u64,
    length: u64,
    hash: [u8; 32],
}

pub struct SrcDaemon {
    // table: managed_largetable::ManagedLargeTable,
    root: std::path::PathBuf,
    remotes: RwLock<HashMap<String, service::SrcServerClient>>,
}

fn mtime(m: &std::fs::Metadata) -> u64 {
    let mt = match m.modified() {
        Ok(m) => m,
        Err(_) => return 0,
    };
    let since_epoch = mt.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as u64
}

fn metadata_compatible(file: service::FileView, m: &std::fs::Metadata) -> bool {
    if !file.get_is_dir() && file.get_length() != m.len() {
        return false;
    }

    if mtime(m) == file.get_mtime() {
        return true;
    }
    false
}

impl SrcDaemon {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
        Self::init(&root)?;

        Ok(Self {
            root,
            remotes: RwLock::new(HashMap::new()),
        })
    }

    pub fn init(root: &std::path::Path) -> std::io::Result<()> {
        // If we already have the root dir set up, skip initialization
        if root.join("changes").join("by_alias").exists() {
            return Ok(());
        }

        std::fs::create_dir_all(root.join("blobs"));
        std::fs::create_dir_all(root.join("links"));
        std::fs::create_dir_all(root.join("changes").join("by_alias"));
        std::fs::create_dir_all(root.join("changes").join("by_dir"));
        std::fs::create_dir_all(root.join("metadata"));
        Ok(())
    }

    pub fn get_blob_path(&self, sha: &[u8]) -> std::path::PathBuf {
        self.root.join("blobs").join(core::fmt_sha(sha))
    }

    pub fn get_blob(&self, sha: &[u8]) -> Option<Vec<u8>> {
        std::fs::read(self.get_blob_path(sha)).ok()
    }

    pub fn get_link_path(&self, alias: &str) -> std::path::PathBuf {
        self.root.join("links").join(alias)
    }

    pub fn get_change_path(&self, alias: &str) -> std::path::PathBuf {
        self.root.join("changes").join("by_alias").join(alias)
    }

    pub fn get_change_dir_path(&self, dir: &std::path::Path) -> std::path::PathBuf {
        let hash = core::fmt_sha(&core::hash_bytes(dir.as_os_str().as_bytes()));
        self.root.join("changes").join("by_dir").join(hash)
    }

    pub fn get_change_by_alias(&self, alias: &str) -> Option<service::Change> {
        let bytes = std::fs::read(self.get_change_path(alias)).ok()?;
        Some(service::Change::decode(&bytes).ok()?)
    }

    pub fn find_unused_alias(&self, original: &str) -> String {
        let mut idx = 1;
        let mut alias = original.to_string();
        while self.get_change_path(&alias).exists() {
            alias = format!("{}-{}", original, idx);
            idx += 1;
        }
        alias
    }

    pub fn get_change_by_dir(&self, dir: &std::path::Path) -> Option<service::Change> {
        for ancestor in dir.ancestors() {
            let path = self.get_change_dir_path(ancestor);
            let alias = match std::fs::read_to_string(path) {
                Ok(a) => a,
                Err(_) => continue,
            };
            return self.get_change_by_alias(&alias);
        }
        None
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

        let connector = Arc::new(bus_rpc::HyperClient::new(hostname.to_string(), port));
        let client = service::SrcServerClient::new(connector);

        self.remotes
            .write()
            .expect("failed to write lock remotes")
            .insert(hostname.to_string(), client.clone());

        Ok(client)
    }

    pub fn validate_basis(&self, basis: service::BasisView) -> std::io::Result<u64> {
        if basis.get_host().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "must provide host name",
            ));
        }

        if basis.get_owner().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "must provide owner",
            ));
        }

        if basis.get_name().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "must provide repository name",
            ));
        }

        if basis.get_change() != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "change basis isn't supported yet",
            ));
        }

        let client = self.get_client(basis.get_host())?;
        let resp = client
            .get_repository(service::GetRepositoryRequest {
                token: String::new(),
                owner: basis.get_owner().to_string(),
                name: basis.get_name().to_string(),
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
                std::io::ErrorKind::NotFound,
                format!("failed to read repository: {}", resp.error_message),
            ));
        }

        Ok(std::cmp::min(resp.index, basis.get_index()))
    }

    fn get_metadata(
        &self,
        basis: service::BasisView,
    ) -> std::io::Result<sstable::SSTableReader<service::FileView>> {
        // Check whether we already downloaded the metadata
        let metadata_path = self
            .root
            .join("metadata")
            .join(basis.get_host())
            .join(basis.get_owner())
            .join(basis.get_name())
            .with_extension("sstable");

        if metadata_path.exists() {
            return Ok(sstable::SSTableReader::from_filename(&metadata_path)?);
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

        Ok(sstable::SSTableReader::from_filename(metadata_path)?)
    }

    fn write_dir(&self, path: &std::path::Path, file: service::FileView) -> std::io::Result<()> {
        std::fs::create_dir_all(path)?;

        let p =
            std::ffi::CString::new(path.as_os_str().as_bytes()).expect("failed to create cstring");
        let times = [
            libc::timeval {
                tv_sec: file.get_mtime() as libc::time_t,
                tv_usec: 0,
            },
            libc::timeval {
                tv_sec: file.get_mtime() as libc::time_t,
                tv_usec: 0,
            },
        ];

        let rc = unsafe { libc::utimes(p.as_ptr(), times.as_ptr()) };
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    fn write_file(
        &self,
        path: &std::path::Path,
        file: service::FileView,
        data: &[u8],
    ) -> std::io::Result<()> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(&path, &data)?;
        let mut f = std::fs::File::create(&path)?;
        f.write_all(&data)?;

        // Set the metadata
        let times = [
            libc::timeval {
                tv_sec: file.get_mtime() as libc::time_t,
                tv_usec: 0,
            },
            libc::timeval {
                tv_sec: file.get_mtime() as libc::time_t,
                tv_usec: 0,
            },
        ];
        let rc = unsafe { libc::futimes(f.as_raw_fd(), times.as_ptr()) };
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    fn diff_from(
        &self,
        root: &std::path::Path,
        path: &std::path::Path,
        metadata: &sstable::SSTableReader<service::FileView>,
        differences: &mut Vec<service::FileDiff>,
    ) -> std::io::Result<()> {
        let get_metadata = |p: &std::path::Path| -> Option<service::FileView> {
            let path_str = p.to_str()?;
            let key = format!("{}/{}", path_str.split("/").count(), path_str);
            let result = metadata.get(&key);
            result
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
                    if metadata_compatible(s, &metadata) {
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
                if metadata_compatible(s, &metadata) {
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
                let content = std::fs::read(&path)?;
                differences.push(service::FileDiff {
                    path: relative_path
                        .to_str()
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_string(),
                    differences: vec![service::ByteDiff {
                        start: 0,
                        end: 0,
                        kind: service::DiffKind::Added,
                        data: content,
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

        let f = std::fs::File::create(self.get_change_path(&alias))?;
        let change = service::Change {
            basis,
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
}

impl service::SrcDaemonServiceHandler for SrcDaemon {
    fn link(&self, req: service::LinkRequest) -> Result<service::LinkResponse, bus::BusRpcError> {
        let link_path = self.get_link_path(&req.alias);
        if link_path.exists() {
            return Ok(service::LinkResponse {
                failed: true,
                error_message: "a link already exists with that alias!".to_string(),
            });
        }

        // Validate that the link is OK
        self.validate_basis(
            service::Basis {
                host: req.host.clone(),
                owner: req.owner.clone(),
                name: req.name.clone(),
                index: 0,
                change: 0,
            }
            .as_view(),
        )
        .map_err(|e| bus::BusRpcError::InternalError(format!("failed to validate link: {}", e)))?;

        let f = std::fs::File::create(link_path).map_err(|e| {
            eprintln!("{:?}", e);
            bus::BusRpcError::InternalError("failed to create link".to_string())
        })?;

        req.encode(&mut std::io::BufWriter::new(f)).map_err(|e| {
            eprintln!("{:?}", e);
            bus::BusRpcError::InternalError("failed to create link".to_string())
        })?;

        Ok(service::LinkResponse::default())
    }

    fn diff(&self, req: service::DiffRequest) -> Result<service::DiffResponse, bus::BusRpcError> {
        // There should be an index for changes based on directory and alias
        // Look up the change there, then find the SSTable for the basis
        let basis = service::Basis::default();

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

        let metadata = self.get_metadata(basis.as_view()).map_err(|e| {
            eprintln!("{:?}", e);
            bus::BusRpcError::InternalError("failed to get metadata".to_string())
        })?;
        let mut differences = Vec::new();

        self.diff_from(&directory, &directory, &metadata, &mut differences)
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("failed to diff".to_string())
            })?;

        Ok(service::DiffResponse {
            files: differences,
            basis,
            ..Default::default()
        })
    }

    fn snapshot(
        &self,
        req: service::SnapshotRequest,
    ) -> Result<service::SnapshotResponse, bus::BusRpcError> {
        todo!()
    }

    fn new_change(
        &self,
        req: service::NewChangeRequest,
    ) -> Result<service::NewChangeResponse, bus::BusRpcError> {
        let directory = std::path::PathBuf::from(&req.dir);
        let directory_str = req.dir;

        // Check that the directory is empty.
        if directory
            .read_dir()
            .map(|mut i| i.next().is_none())
            .unwrap_or(false)
        {
            return Ok(service::NewChangeResponse {
                failed: true,
                error_message: String::from("change directory must be empty!"),
                ..Default::default()
            });
        }

        let index = self.validate_basis(req.basis.as_view()).map_err(|e| {
            bus::BusRpcError::InternalError(format!("failed to validate link: {}", e))
        })?;

        let f = std::fs::File::create(self.get_change_path(&req.alias)).map_err(|e| {
            eprintln!("{:?}", e);
            bus::BusRpcError::InternalError("failed to create change".to_string())
        })?;

        let change = service::Change {
            basis: service::Basis {
                host: req.basis.host.clone(),
                owner: req.basis.owner.clone(),
                name: req.basis.name.clone(),
                change: 0,
                index,
            },
            directory: directory_str.clone(),
        };
        change
            .encode(&mut std::io::BufWriter::new(f))
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("failed to write change".to_string())
            })?;

        std::fs::write(
            self.get_change_dir_path(&std::path::Path::new(&directory_str)),
            req.alias.as_bytes(),
        )
        .map_err(|e| {
            eprintln!("{:?}", e);
            bus::BusRpcError::InternalError("failed to create change dir mapping".to_string())
        })?;

        let metadata = self
            .get_metadata(
                service::Basis {
                    host: req.basis.host.clone(),
                    owner: req.basis.owner.clone(),
                    name: req.basis.name.clone(),
                    change: 0,
                    index,
                }
                .as_view(),
            )
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("failed to get metadata".to_string())
            })?;

        let client = self.get_client(&req.basis.host).map_err(|e| {
            eprintln!("{:?}", e);
            bus::BusRpcError::InternalError("failed to get remote client".to_string())
        })?;

        let mut to_download: HashMap<Vec<u8>, Vec<(String, service::FileView)>> = HashMap::new();
        let mut directories: HashMap<usize, Vec<(std::path::PathBuf, service::FileView)>> =
            HashMap::new();
        for (key, file) in metadata.iter() {
            let (depth, path) = match key.find('/') {
                Some(idx) => {
                    let depth = key[0..idx].parse::<usize>().map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("invalid depth".to_string())
                    })?;
                    (depth, key[idx + 1..].to_string())
                }
                None => {
                    return Ok(service::NewChangeResponse {
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
                Some(b) => self
                    .write_file(&directory.join(path), file, &b)
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to get write file".to_string())
                    })?,
                None => {
                    to_download
                        .entry(file.get_sha().to_owned())
                        .or_insert_with(Vec::new)
                        .push((path, file));
                }
            }

            if to_download.len() >= 250 {
                let resp = client
                    .get_blobs(service::GetBlobsRequest {
                        token: String::new(),
                        shas: to_download.iter().map(|(sha, _)| sha.to_vec()).collect(),
                    })
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to get write file".to_string())
                    })?;

                if resp.failed {
                    return Ok(service::NewChangeResponse {
                        failed: true,
                        error_message: "failed to download blobs!".to_string(),
                        ..Default::default()
                    });
                }

                if resp.blobs.len() != to_download.len() {
                    return Ok(service::NewChangeResponse {
                        failed: true,
                        error_message: "failed to download all blobs!".to_string(),
                        ..Default::default()
                    });
                }

                for blob in &resp.blobs {
                    let files = match to_download.get(&blob.sha) {
                        Some(p) => p,
                        None => {
                            return Ok(service::NewChangeResponse {
                                failed: true,
                                error_message: "got an unexpected blob".to_string(),
                                ..Default::default()
                            });
                        }
                    };

                    for (path, file) in files {
                        self.write_file(&directory.join(path), *file, &blob.data)
                            .map_err(|e| {
                                eprintln!("{:?}", e);
                                bus::BusRpcError::InternalError("failed to write file".to_string())
                            })?;
                    }

                    std::fs::write(self.get_blob_path(&blob.sha), &blob.data).map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to store blob".to_string())
                    });
                }

                to_download.clear();
            }
        }

        if to_download.len() > 0 {
            let resp = client
                .get_blobs(service::GetBlobsRequest {
                    token: String::new(),
                    shas: to_download.iter().map(|(sha, _)| sha.to_vec()).collect(),
                })
                .map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to get write file".to_string())
                })?;

            if resp.failed {
                return Ok(service::NewChangeResponse {
                    failed: true,
                    error_message: "failed to download blobs!".to_string(),
                    ..Default::default()
                });
            }

            println!(
                "response: {}, to_download: {}",
                resp.blobs.len(),
                to_download.len()
            );
            if resp.blobs.len() != to_download.len() {
                return Ok(service::NewChangeResponse {
                    failed: true,
                    error_message: "failed to download all blobs!".to_string(),
                    ..Default::default()
                });
            }

            for blob in &resp.blobs {
                let files = match to_download.get(&blob.sha) {
                    Some(p) => p,
                    None => {
                        return Ok(service::NewChangeResponse {
                            failed: true,
                            error_message: "got an unexpected blob".to_string(),
                            ..Default::default()
                        });
                    }
                };

                for (path, file) in files {
                    self.write_file(&directory.join(path), *file, &blob.data)
                        .map_err(|e| {
                            eprintln!("{:?}", e);
                            bus::BusRpcError::InternalError("failed to write file".to_string())
                        })?;
                }

                std::fs::write(self.get_blob_path(&blob.sha), &blob.data).map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to store blob".to_string())
                });
            }
        }

        let mut dirs: Vec<(usize, Vec<(std::path::PathBuf, service::FileView)>)> =
            directories.into_iter().collect();
        dirs.sort_by_key(|(depth, _)| *depth);
        for (_, items) in dirs.into_iter().rev() {
            for (path, file) in items {
                self.write_dir(&directory.join(path), file).map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to write directory".to_string())
                })?;
            }
        }

        Ok(service::NewChangeResponse {
            dir: directory_str,
            index,
            ..Default::default()
        })
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

    fn daemon() -> SrcDaemon {
        SrcDaemon::new(std::path::PathBuf::from("/tmp/code")).unwrap()
    }

    fn test_checkout() {
        let d = daemon();
        let resp = d
            .new_change(NewChangeRequest {
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
        let d = daemon();
        let resp = d
            .diff(DiffRequest {
                alias: "my-branch".to_string(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(resp.failed, false);
        println!("{:#?}", resp);

        panic!();
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

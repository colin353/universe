use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use bus::{Deserialize, Serialize};

mod checkout;
mod diff;
mod helpers;
mod metadata;

#[derive(Clone)]
pub struct Src {
    root: std::path::PathBuf,
    remotes: Arc<RwLock<HashMap<String, service::SrcServerClient>>>,
}

impl Src {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
        Self::init(&root)?;

        Ok(Self {
            root,
            remotes: Arc::new(RwLock::new(HashMap::new())),
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

    pub fn get_metadata(
        &self,
        basis: service::BasisView,
    ) -> std::io::Result<metadata::Metadata<'static>> {
        // Check whether we already downloaded the metadata
        let metadata_path = self
            .root
            .join("metadata")
            .join(basis.get_host())
            .join(basis.get_owner())
            .join(basis.get_name())
            .join(format!("{}", basis.get_index()))
            .with_extension("sstable");

        if metadata_path.exists() {
            return metadata::Metadata::from_path(metadata_path.clone());
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

        metadata::Metadata::from_path(metadata_path)
    }

    pub fn initialize_repo(
        &self,
        basis: service::Basis,
        path: &std::path::Path,
    ) -> std::io::Result<String> {
        let alias = self.find_unused_alias(&basis.name);

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
        self.set_change_by_alias(&alias, &change)?;
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

        let mut differences = self.diff_from(directory.clone(), directory.clone(), metadata)?;
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

        if req.skip_if_no_changes {
            if diff.files.len() == 0 {
                return Ok(service::SnapshotResponse {
                    skipped: true,
                    ..Default::default()
                });
            }

            if let Ok(Some(s)) = self.get_latest_snapshot(&alias) {
                if s.files == diff.files {
                    return Ok(service::SnapshotResponse {
                        timestamp: s.timestamp,
                        skipped: true,
                        ..Default::default()
                    });
                }
            }
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

    pub fn revert(
        &self,
        path: &std::path::Path,
        metadata: &metadata::Metadata<'static>,
    ) -> std::io::Result<()> {
        match metadata.get(path.to_str().unwrap()) {
            Some(file) => {
                // The file exists in the basis, so return it to that state. First,
                // if the file exists, delete it.
                if path.exists() {
                    if path.is_dir() {
                        std::fs::remove_dir(path).expect("failed to remove directory");
                    } else {
                        std::fs::remove_file(path).expect("failed to remove file");
                    }
                }

                if !self.get_blob_path(file.get_sha()).exists() {
                    unimplemented!("I didn't implement blob cache re-fetching for revert yet!");
                }

                if path.is_dir() {
                    std::fs::create_dir(&path).unwrap();
                } else {
                    std::fs::copy(self.get_blob_path(file.get_sha()), &path).unwrap();
                }
                self.set_mtime(&path, file.get_mtime())
                    .expect("failed to set mtime");
            }
            None => {
                // If the path doesn't exist and shouldn't exist, do nothing. Otherwise delete it
                if path.exists() {
                    if path.is_dir() {
                        std::fs::remove_dir(path).expect("failed to remove directory");
                    } else {
                        std::fs::remove_file(path).expect("failed to remove file");
                    }
                }
            }
        }

        Ok(())
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
            .new_space(CheckoutRequest {
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

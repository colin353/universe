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
    remotes: Arc<RwLock<HashMap<String, service::SrcServerAsyncClient>>>,
}

impl Src {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
        Self::init(&root)?;

        Ok(Self {
            root,
            remotes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn get_client(&self, host: &str) -> std::io::Result<service::SrcServerAsyncClient> {
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
            None => (&host, 8888),
        };

        let connector: Arc<dyn bus::BusAsyncClient> = if hostname == "localhost" {
            Arc::new(bus_rpc::HyperClient::new(hostname.to_string(), port))
        } else {
            Arc::new(bus_rpc::HyperClient::new_tls(hostname.to_string(), port))
        };
        let client = service::SrcServerAsyncClient::new(connector);

        self.remotes
            .write()
            .expect("failed to write lock remotes")
            .insert(hostname.to_string(), client.clone());

        Ok(client)
    }

    pub async fn get_metadata(
        &self,
        basis: service::Basis,
    ) -> std::io::Result<metadata::Metadata<'static>> {
        // Check whether we already downloaded the metadata
        let metadata_path = self
            .root
            .join("metadata")
            .join(&basis.host)
            .join(&basis.owner)
            .join(&basis.name)
            .join(format!("{}", basis.index))
            .with_extension("sstable");

        if metadata_path.exists() {
            return metadata::Metadata::from_path(metadata_path.clone());
        }

        let client = self.get_client(&basis.host)?;
        let token = self.get_identity(&basis.host).unwrap_or_else(String::new);

        let resp = client
            .get_metadata(service::GetMetadataRequest {
                token,
                basis: basis.clone(),
            })
            .await
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

    pub async fn initialize_repo(
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

    pub async fn diff(&self, req: service::DiffRequest) -> std::io::Result<service::DiffResponse> {
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

        let metadata = self.get_metadata(basis.clone()).await?;

        let mut differences = self.diff_from(directory.clone(), directory.clone(), metadata)?;
        differences.sort_by_cached_key(|d| d.path.clone());

        Ok(service::DiffResponse {
            files: differences,
            basis,
            ..Default::default()
        })
    }

    pub async fn snapshot(
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

        let diff = self
            .diff(service::DiffRequest {
                alias: alias.clone(),
                ..Default::default()
            })
            .await?;

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

    pub async fn sync(
        &self,
        alias: &str,
        dry_run: bool,
        conflict_resolutions: &std::collections::HashMap<String, core::ConflictResolutionOverride>,
    ) -> std::io::Result<Result<service::Basis, Vec<(String, core::MergeResult)>>> {
        let mut space = match self.get_change_by_alias(alias) {
            Some(c) => c,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown alias {}", alias),
                ))
            }
        };

        if space.basis.change != 0 {
            unimplemented!("I didn't implement syncing when the basis is another change!");
        }

        let client = self.get_client(&space.basis.host)?;
        let token = self
            .get_identity(&space.basis.host)
            .unwrap_or_else(String::new);
        let repo = client
            .get_repository(service::GetRepositoryRequest {
                token: token.clone(),
                owner: space.basis.owner.clone(),
                name: space.basis.name.clone(),
            })
            .await
            .unwrap();

        // If we're already up to date, nothing to do
        if repo.index == space.basis.index {
            return Ok(Ok(space.basis.clone()));
        }

        let original_metadata = self.get_metadata(space.basis.clone()).await?;

        let mut new_basis = space.basis.clone();
        new_basis.index = repo.index;

        // Reach out and find the diff between the two bases
        let resp = client
            .get_basis_diff(service::GetBasisDiffRequest {
                token: token,
                old: space.basis.clone(),
                new: new_basis.clone(),
            })
            .await
            .unwrap();

        if resp.failed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("failed to diff to new basis: {}", resp.error_message),
            ));
        }

        let mut remote_changes = resp.files;

        let resp = self
            .snapshot(service::SnapshotRequest {
                alias: alias.to_string(),
                message: format!("before syncing to #{}", new_basis.index),
                skip_if_no_changes: true,
                ..Default::default()
            })
            .await?;

        if resp.failed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("failed to snapshot: {}", resp.error_message),
            ));
        }

        let merged_changes = match self.get_latest_snapshot(alias)? {
            Some(s) => {
                let mut local_changes = s.files;
                remote_changes.sort_by(|l, r| l.path.cmp(&r.path));
                local_changes.sort_by(|l, r| l.path.cmp(&r.path));

                let local_iter = local_changes.into_iter();
                let remote_iter = remote_changes.into_iter();
                let mut joined =
                    itertools::JoinedOrderedIterators::new(local_iter, remote_iter, |l, r| {
                        l.path.cmp(&r.path)
                    });

                let mut merged_changes = Vec::new();
                let mut conflicts = Vec::new();
                loop {
                    match joined.next() {
                        (Some(local), Some(remote)) => {
                            let original = original_metadata
                                .get(&local.path)
                                .and_then(|f| self.get_blob(f.get_sha()))
                                .unwrap_or_else(Vec::new);

                            // Possible conflict? Try to auto-resolve/reduce conflicts
                            let merge_result = core::merge(&original, &remote, &local);

                            if let core::MergeResult::Merged(m) = merge_result {
                                merged_changes.push(m);
                            } else {
                                if let Some(resolution) = conflict_resolutions.get(&local.path) {
                                    match resolution {
                                        core::ConflictResolutionOverride::Remote => {
                                            merged_changes.push(remote);
                                        }
                                        core::ConflictResolutionOverride::Local => {
                                            merged_changes.push(local);
                                        }
                                        core::ConflictResolutionOverride::Merged(merged) => {
                                            merged_changes.push(service::FileDiff {
                                                path: local.path,
                                                kind: local.kind,
                                                is_dir: false,
                                                differences: vec![service::ByteDiff {
                                                    start: 0,
                                                    end: original.len() as u32,
                                                    kind: service::DiffKind::Modified,
                                                    data: merged.clone(),
                                                    compression: service::CompressionKind::None,
                                                }],
                                            });
                                        }
                                    }
                                } else {
                                    conflicts.push((local.path, merge_result));
                                }
                            }
                        }
                        (Some(local), None) => {
                            merged_changes.push(local);
                        }
                        (None, Some(remote)) => {
                            merged_changes.push(remote);
                        }
                        (None, None) => break,
                    }
                }
                if !conflicts.is_empty() {
                    return Ok(Err(conflicts));
                }
                merged_changes
            }
            None => {
                // There are no changes on this branch at all, so just return the remote changes.
                remote_changes
            }
        };

        self.apply_snapshot(
            std::path::Path::new(&space.directory),
            space.basis.clone(),
            &merged_changes,
        )
        .await?;

        // Update the space to point to the new basis
        space.basis = new_basis.clone();
        self.set_change_by_alias(alias, &space)?;

        // Record one final snapshot, with the new basis
        let resp = self
            .snapshot(service::SnapshotRequest {
                alias: alias.to_string(),
                message: format!("sync to #{}", new_basis.index),
                skip_if_no_changes: false,
                ..Default::default()
            })
            .await?;

        if resp.failed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("failed to snapshot after merge: {}", resp.error_message),
            ));
        }

        return Ok(Ok(new_basis));
    }
}

use service::*;
use std::collections::HashSet;

pub mod auth;

pub struct SrcServer {
    table: managed_largetable::ManagedLargeTable,
    hostname: String,
    auth: std::sync::Arc<dyn auth::AuthPlugin>,
}

impl SrcServer {
    pub fn new(
        root: std::path::PathBuf,
        hostname: String,
        auth: std::sync::Arc<dyn auth::AuthPlugin>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            table: managed_largetable::ManagedLargeTable::new(root)?,
            hostname,
            auth,
        })
    }

    pub fn auth(&self, token: &str) -> Result<auth::User, String> {
        self.auth.validate(token)
    }

    pub fn get_file(
        &self,
        basis: service::BasisView,
        path: &str,
    ) -> std::io::Result<service::File> {
        if !basis.get_host().is_empty() && basis.get_host() != self.hostname {
            todo!(
                "I don't know how to read from remotes yet (got basis {}, but I'm {})",
                basis.get_host(),
                self.hostname
            );
        }

        self.table
            .read(
                &format!("code/submitted/{}/{}", basis.get_owner(), basis.get_name()),
                &format!("{}/{}", path.split("/").count(), path),
                basis.get_index(),
            )
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?
    }

    pub fn get_blob(&self, basis: service::BasisView, sha: &[u8]) -> std::io::Result<Vec<u8>> {
        if !basis.get_host().is_empty() && basis.get_host() != self.hostname {
            todo!("I don't know how to read from remotes yet!");
        }

        let result: bus::PackedIn<u8> = self
            .table
            .read("code/blobs", &core::fmt_sha(sha), 0)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))??;

        Ok(result.0)
    }

    pub fn get_blob_from_path(
        &self,
        basis: service::BasisView,
        path: &str,
    ) -> std::io::Result<Vec<u8>> {
        let f = self.get_file(basis, path)?;
        self.get_blob(basis, &f.sha)
    }

    pub fn monitor_memory(&self) {
        self.table.monitor_memory();
    }

    fn get_snapshot(
        &self,
        change: &Change,
        timestamp: u64,
    ) -> Result<Option<service::Snapshot>, bus::BusRpcError> {
        let id = if change.original_id != 0 {
            change.original_id
        } else {
            change.id
        };

        let filter = largetable::Filter {
            row: &format!(
                "{}/{}/{}/snapshots",
                change.repo_owner, change.repo_name, id
            ),
            min: "",
            ..Default::default()
        };
        Ok(match self.table.read_range(filter, 0, 10) {
            Ok(m) => m
                .records
                .into_iter()
                .map(|r| Snapshot::from_bytes(&r.data).unwrap())
                .next(),
            Err(e) => {
                return Err(bus::BusRpcError::InternalError(format!(
                    "failed to read from table: {:?}",
                    e
                )));
            }
        })
    }

    fn add_snapshot(
        &self,
        change: &Change,
        snapshot: Snapshot,
    ) -> Result<(), Result<UpdateChangeResponse, bus::BusRpcError>> {
        if snapshot.timestamp == 0 {
            return Err(Ok(UpdateChangeResponse {
                failed: true,
                error_message: format!("Invalid snapshot, timestamp must not be zero",),
                ..Default::default()
            }));
        }

        if snapshot.basis.host != self.hostname {
            return Err(Ok(UpdateChangeResponse {
                failed: true,
                error_message: format!("Invalid basis for change, host must be {}", self.hostname),
                ..Default::default()
            }));
        }

        if snapshot.basis.owner != change.repo_owner || snapshot.basis.name != change.repo_name {
            return Err(Ok(UpdateChangeResponse {
                failed: true,
                error_message: format!("Invalid basis for change, repo didn't match change!",),
                ..Default::default()
            }));
        }

        self.table
            .write(
                format!(
                    "{}/{}/{}/snapshots",
                    change.repo_owner, change.repo_name, change.id
                ),
                core::encode_id(snapshot.timestamp),
                0,
                snapshot,
            )
            .map_err(|e| {
                Err(bus::BusRpcError::InternalError(format!(
                    "failed to write snapshot: {:?}",
                    e
                )))
            })?;

        Ok(())
    }
}

impl service::SrcServerServiceHandler for SrcServer {
    fn create(&self, req: CreateRequest) -> Result<CreateResponse, bus::BusRpcError> {
        let user = match self.auth(&req.token) {
            Ok(u) => u,
            Err(e) => {
                return Ok(CreateResponse {
                    failed: true,
                    error_message: e,
                })
            }
        };

        // TODO: validate that the name is OK
        if req.name.is_empty() {
            return Ok(CreateResponse {
                failed: true,
                error_message: String::from("must provide a valid repository name"),
            });
        }

        self.table
            .write(
                "repos".to_string(),
                format!("{}/{}", user.username, req.name),
                0,
                service::Repository {
                    owner: user.username,
                    name: req.name,
                },
            )
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("failed to write repo".to_string())
            })?;

        Ok(CreateResponse {
            failed: false,
            ..Default::default()
        })
    }

    fn get_repository(
        &self,
        req: GetRepositoryRequest,
    ) -> Result<GetRepositoryResponse, bus::BusRpcError> {
        match self.auth(&req.token) {
            Ok(_) => (),
            Err(e) => {
                return Ok(GetRepositoryResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        if let None =
            self.table
                .read::<bus::Nothing>("repos", &format!("{}/{}", req.owner, req.name), 0)
        {
            return Ok(GetRepositoryResponse {
                failed: true,
                error_message: "that repository doesn't exist".to_string(),
                ..Default::default()
            });
        }

        // Get the latest submitted change
        let filter = largetable::Filter {
            row: &format!("code/submitted_changes/{}/{}", req.owner, req.name),
            spec: "",
            min: "",
            max: "",
        };
        let resp = self.table.read_range(filter, 0, 1).map_err(|e| {
            eprintln!("failed to read range: {:?}", e);
            bus::BusRpcError::InternalError("failed to touch parent folders".to_string())
        })?;

        let index = resp
            .records
            .get(0)
            .map(|r| core::decode_id(&r.key).unwrap_or(0))
            .unwrap_or(0);

        Ok(GetRepositoryResponse {
            index,
            ..Default::default()
        })
    }

    fn submit(&self, req: SubmitRequest) -> Result<SubmitResponse, bus::BusRpcError> {
        match self.auth(&req.token) {
            Ok(_) => (),
            Err(e) => {
                return Ok(SubmitResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        let mut change = match self.table.read(
            &format!("{}/{}/changes", req.repo_owner, req.repo_name),
            &core::encode_id(req.change_id),
            0,
        ) {
            Some(c) => c.map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to read from table: {:?}", e))
            })?,
            None => {
                return Ok(SubmitResponse {
                    failed: true,
                    error_message: "No such change".to_string(),
                    ..Default::default()
                });
            }
        };
        let snapshot = match self.get_snapshot(&change, 0)? {
            Some(s) => s,
            None => {
                return Ok(SubmitResponse {
                    failed: true,
                    error_message: "Snapshot didn't exist".to_string(),
                    ..Default::default()
                });
            }
        };

        // Check that the snapshot matches the snapshot timestamp
        if snapshot.timestamp != req.snapshot_timestamp {
            return Ok(SubmitResponse {
                failed: true,
                error_message: format!(
                    "Snapshot timestamp didn't match (provided {}, expected {})",
                    req.snapshot_timestamp, snapshot.timestamp
                ),
                ..Default::default()
            });
        }

        // TODO: check that the basis is latest
        let submitted_id = self
            .table
            .reserve_id(
                format!("{}/{}/change_ids", change.repo_owner, change.repo_name),
                String::new(),
            )
            .map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to reserve id: {:?}", e))
            })?;

        let mtime = core::timestamp_usec() / 1_000_000;
        let mut modified_paths = HashSet::new();

        for diff in &snapshot.files {
            modified_paths.insert(&diff.path);
            if diff.kind == service::DiffKind::Removed {
                // Delete it
                self.table
                    .delete(
                        format!("code/submitted/{}/{}", req.repo_owner, req.repo_name),
                        format!("{}/{}", diff.path.split("/").count(), diff.path),
                        submitted_id,
                    )
                    .map_err(|e| {
                        eprintln!("failed to delete file: {:?}", e);
                        bus::BusRpcError::InternalError("failed to delete file".to_string())
                    })?;
                continue;
            }

            if diff.is_dir {
                self.table
                    .write(
                        format!("code/submitted/{}/{}", req.repo_owner, req.repo_name),
                        format!("{}/{}", diff.path.split("/").count(), diff.path),
                        submitted_id,
                        service::File {
                            is_dir: true,
                            mtime,
                            sha: vec![],
                            length: 0,
                        },
                    )
                    .map_err(|e| {
                        eprintln!("failed to write dir: {:?}", e);
                        bus::BusRpcError::InternalError("failed to write dir".to_string())
                    })?;
                continue;
            }

            // Figure out what the byte content of the file is from the diff
            let original = match diff.kind {
                service::DiffKind::Modified => self
                    .get_blob_from_path(snapshot.basis.as_view(), &diff.path)
                    .map_err(|e| {
                        eprintln!("failed to get blob: {:?}", e);
                        bus::BusRpcError::InternalError("failed to get blob".to_string())
                    })?,
                _ => Vec::new(),
            };
            let content: Vec<u8> = match core::apply(diff.as_view(), &original) {
                Ok(c) => c,
                Err(e) => {
                    return Ok(SubmitResponse {
                        failed: true,
                        error_message: format!("Failed to apply change to {}: {:?}!", diff.path, e),
                        ..Default::default()
                    });
                }
            };

            let sha = core::hash_bytes(&content);
            let sha_str = core::fmt_sha(&sha);

            // Write to the blobs table if that blob is not yet present
            if self
                .table
                .read::<bus::Nothing>("code/blobs", &sha_str, 0)
                .is_none()
            {
                self.table
                    .write(
                        "code/blobs".to_string(),
                        sha_str,
                        0,
                        bus::PackedOut(&content),
                    )
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to write blob".to_string())
                    })?;
            }

            self.table
                .write(
                    format!("code/submitted/{}/{}", change.repo_owner, change.repo_name),
                    format!("{}/{}", diff.path.split("/").count(), diff.path),
                    submitted_id,
                    service::File {
                        is_dir: false,
                        mtime,
                        sha: sha.into(),
                        length: content.len() as u64,
                    },
                )
                .map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to write file".to_string())
                })?;
        }

        let mut modified_parents = HashSet::new();
        for path in &modified_paths {
            for (idx, _) in path.rmatch_indices("/") {
                modified_parents.insert(&path[0..idx]);
            }
        }

        // Touch all parent folders to update their mtime
        for path in modified_parents {
            self.table
                .write(
                    format!("code/submitted/{}/{}", req.repo_owner, req.repo_name),
                    format!("{}/{}", path.split("/").count(), path),
                    submitted_id,
                    service::File {
                        is_dir: true,
                        mtime,
                        sha: vec![],
                        length: 0,
                    },
                )
                .map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to touch parent folders".to_string())
                })?;
        }

        self.table
            .write(
                format!(
                    "code/submitted_changes/{}/{}",
                    req.repo_owner, req.repo_name
                ),
                core::encode_id(submitted_id),
                0,
                bus::Nothing {},
            )
            .map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to write to table: {:?}", e))
            })?;

        // Mark the change as submitted
        change.status = service::ChangeStatus::Submitted;
        change.submitted_id = submitted_id;
        change.original_id = change.id;
        change.id = submitted_id;

        // Write change back under the original ID and the submitted ID
        self.table
            .write(
                format!("{}/{}/changes", req.repo_owner, req.repo_name),
                core::encode_id(change.original_id),
                0,
                change.clone(),
            )
            .map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to read from table: {:?}", e))
            })?;
        self.table
            .write(
                format!("{}/{}/changes", &req.repo_owner, &req.repo_name),
                core::encode_id(change.submitted_id),
                0,
                change.clone(),
            )
            .map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to read from table: {:?}", e))
            })?;

        Ok(SubmitResponse {
            index: submitted_id,
            ..Default::default()
        })
    }

    fn get_blobs_by_path(
        &self,
        req: GetBlobsByPathRequest,
    ) -> Result<GetBlobsByPathResponse, bus::BusRpcError> {
        match self.auth(&req.token) {
            Ok(_) => (),
            Err(e) => {
                return Ok(GetBlobsByPathResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        let mut shas: Vec<Vec<u8>> = Vec::new();
        let row = format!("code/submitted/{}/{}", req.basis.owner, req.basis.name);
        for path in req.paths {
            let col = format!("{}/{}", path.split("/").count(), path);
            let file = match self
                .table
                .read::<service::File>(&row, &col, req.basis.index)
            {
                Some(Ok(f)) => f,
                Some(Err(e)) => {
                    eprintln!("{:?}", e);
                    return Err(bus::BusRpcError::InternalError(
                        "failed to read from table".to_string(),
                    ));
                }
                None => {
                    return Ok(GetBlobsByPathResponse {
                        failed: true,
                        error_message: format!("could not find blob for {}", path),
                        ..Default::default()
                    })
                }
            };
            shas.push(file.sha);
        }

        let mut blobs = Vec::new();
        for sha in shas {
            let data: bus::PackedIn<u8> =
                match self.table.read("code/blobs", &core::fmt_sha(&sha), 0) {
                    Some(s) => s.map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to get blob".to_string())
                    })?,
                    None => continue,
                };
            blobs.push(service::Blob { sha, data: data.0 })
        }

        Ok(GetBlobsByPathResponse {
            blobs,
            ..Default::default()
        })
    }

    fn get_blobs(&self, req: GetBlobsRequest) -> Result<GetBlobsResponse, bus::BusRpcError> {
        match self.auth(&req.token) {
            Ok(_) => (),
            Err(e) => {
                return Ok(GetBlobsResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        let mut blobs = Vec::new();
        for sha in req.shas {
            let data: bus::PackedIn<u8> =
                match self.table.read("code/blobs", &core::fmt_sha(&sha), 0) {
                    Some(s) => s.map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to get blob".to_string())
                    })?,
                    None => continue,
                };
            blobs.push(service::Blob { sha, data: data.0 })
        }

        Ok(GetBlobsResponse {
            blobs,
            ..Default::default()
        })
    }

    fn get_metadata(
        &self,
        req: GetMetadataRequest,
    ) -> Result<GetMetadataResponse, bus::BusRpcError> {
        match self.auth(&req.token) {
            Ok(_) => (),
            Err(e) => {
                return Ok(GetMetadataResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        let mut data = Vec::new();
        let mut builder = sstable::SSTableBuilder::new(&mut data);

        // Read all of the files at this basis, and emit them into an SSTableBuilder
        let mut min = "".to_string();
        let row = format!("code/submitted/{}/{}", req.basis.owner, req.basis.name);
        let batch_size = 1000;
        loop {
            let filter = largetable::Filter {
                row: &row,
                spec: "",
                min: &min,
                max: "",
            };
            let resp = self
                .table
                .read_range(filter, req.basis.index, batch_size)
                .map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to touch parent folders".to_string())
                })?;

            let count = resp.records.len();
            if count == 0 {
                break;
            }

            min = resp.records[resp.records.len() - 1].key.clone();

            for record in resp.records.into_iter().take(batch_size - 1) {
                builder
                    .write_ordered(&record.key, bus::PackedIn(record.data))
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError(
                            "failed to touch parent folders".to_string(),
                        )
                    })?;
            }

            if count < batch_size {
                break;
            }
        }

        builder.finish().map_err(|e| {
            eprintln!("{:?}", e);
            bus::BusRpcError::InternalError("failed to touch parent folders".to_string())
        })?;

        Ok(GetMetadataResponse {
            data,
            ..Default::default()
        })
    }

    fn update_change(
        &self,
        req: UpdateChangeRequest,
    ) -> Result<UpdateChangeResponse, bus::BusRpcError> {
        let user = match self.auth(&req.token) {
            Ok(u) => u,
            Err(e) => {
                return Ok(UpdateChangeResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        // Check that the repository exists
        match self.table.read::<bus::Nothing>(
            "repos",
            &format!("{}/{}", req.change.repo_owner, req.change.repo_name),
            0,
        ) {
            Some(r) => r.map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to read from table: {:?}", e))
            })?,
            None => {
                return Ok(UpdateChangeResponse {
                    failed: true,
                    error_message: format!(
                        "No such repository: {}/{}",
                        req.change.repo_owner, req.change.repo_name
                    )
                    .to_string(),
                    ..Default::default()
                })
            }
        };

        // Check if the change already exists
        if req.change.id != 0 {
            let mut existing_change = match self.table.read::<service::Change>(
                &format!("{}/{}/changes", req.change.repo_owner, req.change.repo_name),
                &core::encode_id(req.change.id),
                0,
            ) {
                Some(c) => c.map_err(|e| {
                    bus::BusRpcError::InternalError(format!("failed to read from table: {:?}", e))
                })?,
                None => {
                    return Ok(UpdateChangeResponse {
                        failed: true,
                        error_message: "No such change".to_string(),
                        ..Default::default()
                    });
                }
            };

            if existing_change.owner != user.username {
                return Ok(UpdateChangeResponse {
                    failed: true,
                    error_message: format!("No permission to modify change",),
                    ..Default::default()
                });
            }

            // Update any fields that can be updated
            if !req.change.description.is_empty() {
                existing_change.description = req.change.description;
            }

            // You can set a change as archived, but only from pending state
            if req.change.status == service::ChangeStatus::Archived
                && existing_change.status == service::ChangeStatus::Pending
            {
                existing_change.status = req.change.status;
            }

            self.table
                .write(
                    format!(
                        "{}/{}/changes",
                        existing_change.repo_owner, existing_change.repo_name
                    ),
                    core::encode_id(existing_change.id),
                    0,
                    existing_change.clone(),
                )
                .map_err(|e| {
                    bus::BusRpcError::InternalError(format!("failed to write to table: {:?}", e))
                })?;

            // Add snapshot, if there's one to add
            if req.snapshot.timestamp != 0 {
                if let Err(e) = self.add_snapshot(&existing_change, req.snapshot) {
                    return e;
                }
            }

            return Ok(UpdateChangeResponse {
                id: req.change.id,
                ..Default::default()
            });
        }

        // We're creating the change from scratch. Validate it, reserve an id, and write
        if req.snapshot.basis.host != self.hostname {
            return Ok(UpdateChangeResponse {
                failed: true,
                error_message: format!("Invalid basis for change, host must be {}", self.hostname),
                ..Default::default()
            });
        }

        if req.snapshot.basis.owner != req.change.repo_owner
            || req.snapshot.basis.name != req.change.repo_name
        {
            return Ok(UpdateChangeResponse {
                failed: true,
                error_message: format!("Invalid basis for change, repo didn't match change!",),
                ..Default::default()
            });
        }

        if req.snapshot.timestamp == 0 {
            return Ok(UpdateChangeResponse {
                failed: true,
                error_message: format!("Invalid snapshot, timestamp must not be zero",),
                ..Default::default()
            });
        }

        let id = self
            .table
            .reserve_id(
                format!(
                    "{}/{}/change_ids",
                    req.change.repo_owner, req.change.repo_name
                ),
                String::new(),
            )
            .map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to reserve id: {:?}", e))
            })?;

        let mut change = req.change;
        change.id = id;
        change.owner = user.username;
        change.status = service::ChangeStatus::Pending;

        if let Err(e) = self.add_snapshot(&change, req.snapshot) {
            return e;
        };

        self.table
            .write(
                format!("{}/{}/changes", change.repo_owner, change.repo_name),
                core::encode_id(id),
                0,
                change.clone(),
            )
            .map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to write to table: {:?}", e))
            })?;

        // Update the user index
        self.table
            .write(
                format!("{}/changes", change.owner),
                format!(
                    "{}/{}/{}",
                    change.repo_owner,
                    change.repo_name,
                    core::encode_id(change.id)
                ),
                0,
                bus::Nothing {},
            )
            .map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to write to table: {:?}", e))
            })?;

        Ok(UpdateChangeResponse {
            id,
            ..Default::default()
        })
    }

    fn list_changes(
        &self,
        req: ListChangesRequest,
    ) -> Result<ListChangesResponse, bus::BusRpcError> {
        let user = match self.auth(&req.token) {
            Ok(u) => u,
            Err(e) => {
                return Ok(ListChangesResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                });
            }
        };

        let req_filter = |c: &service::Change| {
            if req.status != service::ChangeStatus::Unknown && req.status != c.status {
                return false;
            }
            true
        };

        let changes = if !req.owner.is_empty() {
            let row = format!("{}/changes", user.username);
            let filter = largetable::Filter {
                row: &row,
                spec: "",
                min: &req.starting_from,
                max: "",
            };
            let resp = self.table.read_range(filter, 0, 1000).map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("read range error".to_string())
            })?;

            let expected_prefix = if !req.repo_name.is_empty() && !req.repo_owner.is_empty() {
                format!("{}/{}", req.repo_owner, req.repo_name)
            } else {
                String::new()
            };
            let mut changes = Vec::new();
            for record in resp.records {
                if !record.key.starts_with(&expected_prefix) {
                    continue;
                }

                let components: Vec<_> = record.key.split("/").collect();
                if components.len() != 3 {
                    return Err(bus::BusRpcError::InternalError(
                        "read incorrect record format in user change index!".to_string(),
                    ));
                }

                let row = format!("{}/{}/changes", components[0], components[1]);
                let col = components[2];

                if let Some(c) = self.table.read(&row, &col, 0) {
                    let c = c.map_err(|e| {
                        bus::BusRpcError::InternalError(format!(
                            "failed to read from table: {:?}",
                            e
                        ))
                    })?;
                    if req_filter(&c) {
                        changes.push(c);
                    }
                }

                if changes.len() == req.limit as usize {
                    break;
                }
            }
            changes
        } else if !req.repo_name.is_empty() && !req.repo_owner.is_empty() {
            let row = format!("{}/{}/changes", req.repo_owner, req.repo_name);
            let start_id =
                if !req.starting_from.is_empty() {
                    let components: Vec<_> = req.starting_from.split("/").collect();
                    if components.len() != 3 {
                        return Err(bus::BusRpcError::InternalError(
                            "starting_from field must be in the format <owner>/<repo>/<change id>"
                                .to_string(),
                        ));
                    }
                    match components[2].parse::<u64>() {
                        Ok(id) => core::encode_id(id),
                        Err(_) => return Err(bus::BusRpcError::InternalError(
                            "starting_from field must be in the format <owner>/<repo>/<change id>"
                                .to_string(),
                        )),
                    }
                } else {
                    String::new()
                };

            let filter = largetable::Filter {
                row: &row,
                spec: "",
                min: &req.starting_from,
                max: "",
            };
            let resp = self.table.read_range(filter, 0, 1000).map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("failed to read from table".to_string())
            })?;

            let mut changes = Vec::new();
            for record in resp.records {
                let change = match service::Change::from_bytes(&record.data) {
                    Ok(c) => c,
                    Err(_) => {
                        return Err(bus::BusRpcError::InternalError(
                            "unable to decode change!".to_string(),
                        ))
                    }
                };
                if req_filter(&change) {
                    changes.push(change);
                }
            }
            changes
        } else {
            return Ok(ListChangesResponse {
                failed: true,
                error_message: "a repo name or user must be specified".to_string(),
                ..Default::default()
            });
        };

        Ok(ListChangesResponse {
            changes,
            ..Default::default()
        })
    }

    fn get_change(&self, req: GetChangeRequest) -> Result<GetChangeResponse, bus::BusRpcError> {
        let username = match self.auth(&req.token) {
            Ok(u) => u,
            Err(e) => {
                return Ok(GetChangeResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        let change = match self.table.read(
            &format!("{}/{}/changes", req.repo_owner, req.repo_name),
            &core::encode_id(req.id),
            0,
        ) {
            Some(c) => c.map_err(|e| {
                bus::BusRpcError::InternalError(format!("failed to read from table: {:?}", e))
            })?,
            None => {
                return Ok(GetChangeResponse {
                    failed: true,
                    error_message: "No such change".to_string(),
                    ..Default::default()
                });
            }
        };

        let snapshot = self.get_snapshot(&change, 0)?;

        Ok(GetChangeResponse {
            change,
            latest_snapshot: snapshot.unwrap_or_else(Snapshot::default),
            ..Default::default()
        })
    }

    fn get_basis_diff(
        &self,
        req: GetBasisDiffRequest,
    ) -> Result<GetBasisDiffResponse, bus::BusRpcError> {
        match self.auth(&req.token) {
            Ok(u) => u,
            Err(e) => {
                return Ok(GetBasisDiffResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        if !req.new.host.is_empty() && req.new.host != self.hostname {
            return Ok(GetBasisDiffResponse {
                failed: true,
                error_message: "Can't check basis diff from a different host!".to_string(),
                ..Default::default()
            });
        }

        if req.new.host != req.old.host {
            return Ok(GetBasisDiffResponse {
                failed: true,
                error_message: "Can't check basis diff across hosts!".to_string(),
                ..Default::default()
            });
        }

        if req.new.owner != req.old.owner || req.new.name != req.old.name {
            return Ok(GetBasisDiffResponse {
                failed: true,
                error_message: "Can't check basis diff across repos!".to_string(),
                ..Default::default()
            });
        }

        if req.old.change != 0 || req.new.change != 0 {
            return Ok(GetBasisDiffResponse {
                failed: true,
                error_message: "Checking basis across changes is not supported yet".to_string(),
                ..Default::default()
            });
        }

        if req.old.index > req.new.index {
            return Ok(GetBasisDiffResponse {
                failed: true,
                error_message: "Checking reverse diff is not supported yet".to_string(),
                ..Default::default()
            });
        }

        let mut accumulated_changes = std::collections::HashMap::new();
        let row = format!("{}/{}/changes", req.new.owner, req.new.name);
        for id in req.old.index + 1..req.new.index {
            let change = match self.table.read(&row, &core::encode_id(id), 0) {
                Some(c) => c.map_err(|e| {
                    bus::BusRpcError::InternalError(format!("failed to read from table: {:?}", e))
                })?,
                None => continue,
            };
            let snapshot = match self.get_snapshot(&change, 0)? {
                Some(s) => s,
                None => continue,
            };

            for file in snapshot.files {
                if accumulated_changes.contains_key(&file.path) {
                    accumulated_changes.insert(file.path, None);
                } else {
                    accumulated_changes.insert(file.path.clone(), Some(file));
                }
            }
        }

        let mut output = Vec::new();
        let row = format!("code/submitted/{}/{}", &req.new.owner, req.new.name);
        for (path, maybe_file) in accumulated_changes {
            if let Some(file) = maybe_file {
                output.push(file);
            } else {
                // Get the old file and the new file, and compute a new diff
                let old = self.table.read::<File>(
                    &row,
                    &format!("{}/{}", path.split("/").count(), path),
                    req.old.index,
                );
                let new = self.table.read::<File>(
                    &row,
                    &format!("{}/{}", path.split("/").count(), path),
                    req.new.index,
                );

                match (old, new) {
                    (Some(Ok(old_f)), Some(Ok(new_f))) => {
                        let old_file_content =
                            self.get_blob(req.old.as_view(), &new_f.sha).map_err(|e| {
                                bus::BusRpcError::InternalError(format!(
                                    "failed to get blob: {:?}",
                                    e
                                ))
                            })?;
                        let new_file_content =
                            self.get_blob(req.new.as_view(), &new_f.sha).map_err(|e| {
                                bus::BusRpcError::InternalError(format!(
                                    "failed to get blob: {:?}",
                                    e
                                ))
                            })?;

                        // Emit diff between the files
                        let bytediffs = core::diff(&old_file_content, &new_file_content);
                        output.push(FileDiff {
                            path: path,
                            differences: bytediffs,
                            is_dir: new_f.is_dir,
                            kind: service::DiffKind::Modified,
                        });
                    }
                    (Some(Ok(old_f)), None) => {
                        // Emit deletion
                        output.push(FileDiff {
                            path: path,
                            differences: vec![],
                            is_dir: old_f.is_dir,
                            kind: service::DiffKind::Removed,
                        });
                    }
                    (None, Some(Ok(new_f))) => {
                        // Emit add
                        if new_f.is_dir {
                            output.push(FileDiff {
                                path: path,
                                differences: vec![],
                                is_dir: new_f.is_dir,
                                kind: service::DiffKind::Added,
                            });
                            continue;
                        }

                        // Compress file content and emit add
                        let file_content =
                            self.get_blob(req.new.as_view(), &new_f.sha).map_err(|e| {
                                bus::BusRpcError::InternalError(format!(
                                    "failed to get blob: {:?}",
                                    e
                                ))
                            })?;
                        output.push(FileDiff {
                            path: path,
                            differences: vec![ByteDiff {
                                start: 0,
                                end: 0,
                                kind: DiffKind::Added,
                                data: core::compress(&file_content),
                                compression: CompressionKind::LZ4,
                            }],
                            is_dir: new_f.is_dir,
                            kind: DiffKind::Added,
                        });
                    }
                    (None, None) => {
                        // Created but then deleted, ignore
                    }
                    _ => {
                        // Bus error ocurred
                        return Ok(GetBasisDiffResponse {
                            failed: true,
                            error_message: "Failed to connect to largetable".to_string(),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        Ok(GetBasisDiffResponse {
            files: output,
            ..Default::default()
        })
    }

    fn discover_auth(
        &self,
        _: service::DiscoverAuthRequest,
    ) -> Result<service::DiscoverAuthResponse, bus::BusRpcError> {
        Ok(self.auth.discover())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus::{Deserialize, Serialize};

    fn setup() -> SrcServer {
        let path = std::path::PathBuf::from("/tmp/asdf");
        std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path);
        SrcServer::new(
            path,
            "localhost:4959".to_string(),
            std::sync::Arc::new(auth::FakeAuthPlugin::new()),
        )
        .unwrap()
    }

    fn full_setup() -> SrcServer {
        let s = setup();
        s.create(CreateRequest {
            token: String::new(),
            name: "example".to_string(),
        })
        .unwrap();
        s
    }

    //#[test]
    fn test_create_repo() {
        let s = setup();
        let resp = s
            .get_repository(GetRepositoryRequest {
                token: String::new(),
                owner: "colin".to_string(),
                name: "universe".to_string(),
            })
            .unwrap();
        assert_eq!(resp.failed, true);

        let resp = s
            .create(CreateRequest {
                token: String::new(),
                name: "universe".to_string(),
            })
            .unwrap();
        assert_eq!(resp.failed, false);

        let resp = s
            .get_repository(GetRepositoryRequest {
                token: String::new(),
                owner: "colin".to_string(),
                name: "universe".to_string(),
            })
            .unwrap();
        assert_eq!(resp.failed, false);
        assert_eq!(resp.index, 1);
    }

    fn test_create_change() {
        let s = setup();
        s.create(CreateRequest {
            token: String::new(),
            name: "example".to_string(),
        })
        .unwrap();

        let r = s
            .update_change(UpdateChangeRequest {
                token: String::new(),
                change: Change {
                    description: "do something".to_string(),
                    repo_owner: "colin".to_string(),
                    repo_name: "example".to_string(),
                    ..Default::default()
                },
                snapshot: Snapshot {
                    timestamp: 123,
                    basis: Basis {
                        host: "localhost:4959".to_string(),
                        owner: "colin".to_string(),
                        name: "example".to_string(),
                        ..Default::default()
                    },
                    files: vec![],
                    message: String::new(),
                },
            })
            .unwrap();
        assert!(!r.failed);
        assert_eq!(r.id, 1);
    }

    #[test]
    fn test_get_change() {
        let s = setup();
        s.create(CreateRequest {
            token: String::new(),
            name: "example".to_string(),
        });

        let response = s
            .update_change(UpdateChangeRequest {
                token: String::new(),
                change: Change {
                    description: "do something".to_string(),
                    repo_owner: "colin".to_string(),
                    repo_name: "example".to_string(),
                    ..Default::default()
                },
                snapshot: Snapshot {
                    timestamp: 123,
                    basis: Basis {
                        host: "localhost:4959".to_string(),
                        owner: "colin".to_string(),
                        name: "example".to_string(),
                        ..Default::default()
                    },
                    files: vec![],
                    message: String::new(),
                },
            })
            .unwrap();
        let id = response.id;

        let response = s
            .get_change(GetChangeRequest {
                token: String::new(),
                repo_owner: "colin".to_string(),
                repo_name: "example".to_string(),
                id: id,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(response.error_message, String::new());
        assert_eq!(response.failed, false);
        assert_eq!(response.change.id, id);
        assert_eq!(&response.change.description, "do something");
    }

    #[test]
    fn test_list_changes() {
        let s = setup();
        s.create(CreateRequest {
            token: String::new(),
            name: "example".to_string(),
        });

        s.update_change(UpdateChangeRequest {
            token: String::new(),
            change: Change {
                description: "do something".to_string(),
                repo_owner: "colin".to_string(),
                repo_name: "example".to_string(),
                ..Default::default()
            },
            snapshot: Snapshot {
                timestamp: 123,
                basis: Basis {
                    host: "localhost:4959".to_string(),
                    owner: "colin".to_string(),
                    name: "example".to_string(),
                    ..Default::default()
                },
                files: vec![],
                message: String::new(),
            },
        })
        .unwrap();

        let response = s
            .list_changes(ListChangesRequest {
                token: String::new(),
                owner: "colin".to_string(),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(response.error_message, String::new());
        assert_eq!(response.failed, false);
        assert_eq!(response.changes.len(), 1);
        assert_eq!(&response.changes[0].description, "do something");

        let response = s
            .list_changes(ListChangesRequest {
                token: String::new(),
                repo_owner: "colin".to_string(),
                repo_name: "example".to_string(),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(response.error_message, String::new());
        assert_eq!(response.failed, false);
        assert_eq!(response.changes.len(), 1);
        assert_eq!(&response.changes[0].description, "do something");
    }

    #[test]
    fn test_encode_decode_snapshot() {
        let req = service::SubmitRequest {
            token: String::new(),
            repo_owner: "colin".to_string(),
            repo_name: "example".to_string(),
            change_id: 1,
            snapshot_timestamp: 1671910873667006,
        };
        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();

        let r2 = service::SubmitRequest::from_bytes(&buf).unwrap();
        assert_eq!(r2, req);
        assert_eq!(r2.snapshot_timestamp, 16719);
    }
}

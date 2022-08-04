use service::*;
use std::collections::HashSet;

pub struct SrcServer {
    table: managed_largetable::ManagedLargeTable,
}

impl SrcServer {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            table: managed_largetable::ManagedLargeTable::new(root)?,
        })
    }

    pub fn auth(&self, _token: &str) -> Result<String, String> {
        Ok(String::from("colin"))
    }

    pub fn get_file(
        &self,
        basis: service::BasisView,
        path: &str,
    ) -> std::io::Result<service::File> {
        if !basis.get_host().is_empty() {
            todo!("I don't know how to read from remotes yet!");
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
        if !basis.get_host().is_empty() {
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
                format!("{}/{}", user, req.name),
                0,
                service::Repository {
                    owner: user,
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

        Ok(GetRepositoryResponse {
            failed: false,
            index: 1,
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

        // TODO: check that the basis is valid
        // TODO: check that the index is latest

        let index = 2;

        let mtime = managed_largetable::timestamp_usec() / 1_000_000;
        let mut modified_paths = HashSet::new();

        for diff in &req.files {
            modified_paths.insert(&diff.path);
            if diff.kind == service::DiffKind::Removed {
                // Delete it
                self.table
                    .delete(
                        format!("code/submitted/{}/{}", req.basis.owner, req.basis.name),
                        format!("{}/{}", diff.path.split("/").count(), diff.path),
                        index,
                    )
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to delete file".to_string())
                    })?;
                continue;
            }

            if diff.is_dir {
                self.table
                    .write(
                        format!("code/submitted/{}/{}", req.basis.owner, req.basis.name),
                        format!("{}/{}", diff.path.split("/").count(), diff.path),
                        index,
                        service::File {
                            is_dir: true,
                            mtime,
                            sha: vec![],
                        },
                    )
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to write dir".to_string())
                    })?;
                continue;
            }

            // Figure out what the byte content of the file is from the diff
            let content: Vec<u8> = if diff.kind == service::DiffKind::Added {
                diff.differences[0].data.clone()
            } else {
                let original = self
                    .get_blob_from_path(req.basis.as_view(), &diff.path)
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to get blob".to_string())
                    })?;

                // Reconstruct the new document from the diff representation
                let mut content = Vec::new();
                let mut idx: usize = 0;
                for bd in &diff.differences {
                    content.extend_from_slice(&original[idx..bd.start as usize]);
                    if bd.kind == service::DiffKind::Added {
                        content.extend_from_slice(&bd.data);
                    } else {
                        idx = bd.end as usize;
                    }
                }
                content.extend_from_slice(&original[idx..]);
                content
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
                    format!("code/submitted/{}/{}", req.basis.owner, req.basis.name),
                    format!("{}/{}", diff.path.split("/").count(), diff.path),
                    index,
                    service::File {
                        is_dir: false,
                        mtime,
                        sha: sha.into(),
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
                    format!("code/submitted/{}/{}", req.basis.owner, req.basis.name),
                    format!("{}/{}", path.split("/").count(), path),
                    index,
                    service::File {
                        is_dir: true,
                        mtime,
                        sha: vec![],
                    },
                )
                .map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to touch parent folders".to_string())
                })?;
        }

        Ok(SubmitResponse {
            index,
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
        loop {
            let filter = largetable::Filter {
                row: &row,
                spec: "",
                min: &min,
                max: "",
            };
            let resp = self
                .table
                .read_range(filter, req.basis.index, 1000)
                .map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to touch parent folders".to_string())
                })?;

            let count = resp.records.len();
            min = resp.records[resp.records.len() - 1].key.clone();

            for record in resp.records {
                builder
                    .write_ordered(&record.key, bus::PackedIn(record.data))
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError(
                            "failed to touch parent folders".to_string(),
                        )
                    })?;
            }

            if count < 1000 {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SrcServer {
        let path = std::path::PathBuf::from("/tmp/asdf");
        std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path);
        SrcServer::new(path).unwrap()
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

    #[test]
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

    #[test]
    fn test_submit_change() {
        let s = full_setup();
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
                        data: vec![0, 1, 2, 3, 4],
                        ..Default::default()
                    }],
                },
                FileDiff {
                    path: "dir/b.txt".to_string(),
                    kind: DiffKind::Added,
                    is_dir: false,
                    differences: vec![ByteDiff {
                        data: vec![4, 3, 2, 1, 0],
                        ..Default::default()
                    }],
                },
            ],
        })
        .unwrap();

        // Check on blobs
        let desired_sha = core::hash_bytes(&[0, 1, 2, 3, 4]);
        let blobs = s
            .get_blobs(GetBlobsRequest {
                token: String::new(),
                shas: vec![desired_sha.into()],
            })
            .unwrap();

        assert_eq!(blobs.blobs.len(), 1);
        assert_eq!(&blobs.blobs[0].sha, &desired_sha);
        assert_eq!(&blobs.blobs[0].data, &[0, 1, 2, 3, 4]);

        // Read metadata
        let metadata_sstable = s
            .get_metadata(GetMetadataRequest {
                token: String::new(),
                basis: Basis {
                    owner: "colin".to_string(),
                    name: "example".to_string(),
                    ..Default::default()
                },
            })
            .unwrap();

        let sst = sstable::SSTableReader::<File>::from_bytes(&metadata_sstable.data).unwrap();
        let file = sst.get("1/a.txt").unwrap();
        assert_eq!(&file.sha, &desired_sha);
        assert!(file.mtime > 1659000000);
        assert_eq!(file.is_dir, false);

        let file = sst.get("1/dir").unwrap();
        assert_eq!(&file.sha, &[]);
        assert!(file.mtime > 1659000000);
        assert_eq!(file.is_dir, true);

        let file = sst.get("2/dir/b.txt").unwrap();
        assert!(file.mtime > 1659000000);
        assert_eq!(file.is_dir, false);
    }
}

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;

struct SrcDaemon {
    table: managed_largetable::ManagedLargeTable,
    root: std::path::PathBuf,
    remotes: RwLock<HashMap<String, service::SrcServerClient>>,
}

impl SrcDaemon {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(root.join("db"));
        std::fs::create_dir_all(root.join("metadata"));

        Ok(Self {
            table: managed_largetable::ManagedLargeTable::new(root.join("db"))?,
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

        println!("connecting on {}:{}", hostname, port);

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
        println!("write dir: {:?}", path);
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
        println!("write file: {:?}", path);
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
}

impl service::SrcDaemonServiceHandler for SrcDaemon {
    fn link(&self, req: service::LinkRequest) -> Result<service::LinkResponse, bus::BusRpcError> {
        if self
            .table
            .read::<bus::Nothing>("links", &req.alias, 0)
            .is_some()
        {
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

        self.table
            .write("links".to_string(), req.alias.clone(), 0, req)
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("failed to create change".to_string())
            })?;

        Ok(service::LinkResponse::default())
    }

    fn diff(&self, req: service::DiffRequest) -> Result<service::DiffResponse, bus::BusRpcError> {
        todo!()
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

        self.table
            .write(
                "changes".to_string(),
                req.alias,
                0,
                service::Change {
                    basis: service::Basis {
                        host: req.basis.host.clone(),
                        owner: req.basis.owner.clone(),
                        name: req.basis.name.clone(),
                        change: 0,
                        index,
                    },
                    directory: directory_str.clone(),
                },
            )
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("failed to create change".to_string())
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

            if let Some(Ok(b)) =
                self.table
                    .read::<bus::PackedIn<u8>>("blobs", &core::fmt_sha(file.get_sha()), 0)
            {
                self.write_file(&directory.join(path), file, &b.0)
                    .map_err(|e| {
                        eprintln!("{:?}", e);
                        bus::BusRpcError::InternalError("failed to get write file".to_string())
                    })?;
            } else {
                to_download
                    .entry(file.get_sha().to_owned())
                    .or_insert_with(Vec::new)
                    .push((path, file));
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
            }
        }

        // Finally, update the mtime on all directories. This must be done last, since the mtime is
        // affected by writing the files inside of the directories!
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

    #[test]
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

        panic!();
    }

    #[test]
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
    }
}

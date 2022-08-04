use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use std::io::Write;
use std::os::unix::io::AsRawFd;

struct SrcDaemon {
    table: managed_largetable::ManagedLargeTable,
    root: std::path::PathBuf,
    remotes: RwLock<HashMap<String, service::SrcServerClient>>,
}

impl SrcDaemon {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
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

        let connector = Arc::new(bus_rpc::HyperClient::new(String::from("127.0.0.1"), 4521));
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
            .with_extension(".sstable");

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

        std::fs::write(&metadata_path, &resp.data)?;

        Ok(sstable::SSTableReader::from_filename(metadata_path)?)
    }

    fn write_file(
        &self,
        path: &std::path::Path,
        file: service::FileView,
        data: &[u8],
    ) -> std::io::Result<()> {
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

        let mut to_download: Vec<(Vec<u8>, String, service::FileView)> = Vec::new();
        for (key, file) in metadata.iter() {
            let path = match key.find('/') {
                Some(idx) => key[idx + 1..].to_string(),
                None => {
                    return Ok(service::NewChangeResponse {
                        failed: true,
                        error_message: "invalid metadata path!".to_string(),
                        ..Default::default()
                    });
                }
            };

            if let Some(Ok(b)) =
                self.table
                    .read::<bus::PackedIn<u8>>("blobs", &core::fmt_sha(file.get_sha()), 0)
            {
                self.write_file(&directory, file, &b.0).map_err(|e| {
                    eprintln!("{:?}", e);
                    bus::BusRpcError::InternalError("failed to get write file".to_string())
                })?;
            } else {
                to_download.push((file.get_sha().to_vec(), path, file));
            }

            if to_download.len() >= 250 {
                let resp = client
                    .get_blobs(service::GetBlobsRequest {
                        token: String::new(),
                        shas: to_download.iter().map(|(sha, _, _)| sha.to_vec()).collect(),
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

                for ((sha, path, file), blob) in to_download.iter().zip(resp.blobs.iter()) {
                    self.write_file(path.as_ref(), *file, &blob.data)
                        .map_err(|e| {
                            eprintln!("{:?}", e);
                            bus::BusRpcError::InternalError("failed to write file".to_string())
                        })?;
                }
            }
        }

        Ok(service::NewChangeResponse {
            dir: directory_str,
            index,
            ..Default::default()
        })
    }
}

pub fn main() {}

use std::os::unix::ffi::OsStrExt;

use bus::{Deserialize, Serialize};

fn mtime(m: &std::fs::Metadata) -> u64 {
    let mt = match m.modified() {
        Ok(m) => m,
        Err(_) => return 0,
    };
    let since_epoch = mt.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as u64
}

pub fn metadata_compatible(file: service::FileView, m: &std::fs::Metadata) -> bool {
    if file.get_is_dir() {
        return false;
    }

    if file.get_length() != m.len() {
        return false;
    }

    mtime(m) == file.get_mtime()
}

impl crate::Src {
    pub(crate) fn init(root: &std::path::Path) -> std::io::Result<()> {
        // If we already have the root dir set up, skip initialization
        if root.join("changes").join("by_alias").exists() {
            return Ok(());
        }

        // OK to ignore failure here, since that means the dir exists already
        std::fs::create_dir_all(root.join("blobs")).ok();
        std::fs::create_dir_all(root.join("changes").join("by_alias")).ok();
        std::fs::create_dir_all(root.join("changes").join("by_dir")).ok();
        std::fs::create_dir_all(root.join("metadata")).ok();
        std::fs::create_dir_all(root.join("identity")).ok();
        Ok(())
    }

    pub(crate) fn get_snapshot_path(&self, alias: &str, ts: u64) -> std::path::PathBuf {
        self.get_change_path(&alias)
            .join(format!("{}.snapshot", ts))
    }

    pub(crate) fn get_blob_path(&self, sha: &[u8]) -> std::path::PathBuf {
        self.root.join("blobs").join(core::fmt_sha(sha))
    }

    pub fn get_blob(&self, sha: &[u8]) -> Option<Vec<u8>> {
        std::fs::read(self.get_blob_path(sha)).ok()
    }

    pub fn get_change_metadata_path(&self, alias: &str) -> std::path::PathBuf {
        self.root
            .join("changes")
            .join("by_alias")
            .join(alias)
            .join("metadata")
    }

    pub(crate) fn get_change_path(&self, alias: &str) -> std::path::PathBuf {
        self.root.join("changes").join("by_alias").join(alias)
    }

    pub fn get_change_dir_path(&self, dir: &std::path::Path) -> std::path::PathBuf {
        let hash = core::fmt_sha(&core::hash_bytes(dir.as_os_str().as_bytes()));
        self.root.join("changes").join("by_dir").join(hash)
    }

    pub fn get_change_by_alias(&self, alias: &str) -> Option<service::Space> {
        if alias.is_empty() {
            return None;
        }
        let bytes = std::fs::read(self.get_change_metadata_path(alias)).ok()?;
        Some(service::Space::decode(&bytes).ok()?)
    }

    pub fn get_change_alias_by_dir(&self, dir: &std::path::Path) -> Option<String> {
        for ancestor in dir.ancestors() {
            let path = self.get_change_dir_path(ancestor);
            let alias = match std::fs::read_to_string(path) {
                Ok(a) => a,
                Err(_) => continue,
            };
            return Some(alias);
        }
        None
    }

    pub fn get_spaces(&self) -> impl Iterator<Item = (String, service::Space)> {
        std::fs::read_dir(self.root.join("changes").join("by_alias"))
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| {
                let ft = entry.metadata().unwrap().file_type();
                ft.is_dir()
            })
            .filter_map(|entry| {
                let bytes: Vec<_> = match std::fs::read(entry.path().join("metadata")) {
                    Ok(b) => b,
                    Err(_) => {
                        // Invalid state, repair by deleting the by_alias dir
                        std::fs::remove_dir_all(entry.path()).ok();
                        return None;
                    }
                };
                Some((
                    entry.file_name().into_string().unwrap(),
                    service::Space::decode(&bytes).unwrap(),
                ))
            })
    }

    pub fn get_change_by_dir(&self, dir: &std::path::Path) -> Option<service::Space> {
        let alias = self.get_change_alias_by_dir(dir)?;
        self.get_change_by_alias(&alias)
    }

    pub fn set_change_by_alias(&self, alias: &str, space: &service::Space) -> std::io::Result<()> {
        if alias.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "must provide non-empty alias",
            ));
        }
        if !space.directory.is_empty() {
            std::fs::write(
                self.get_change_dir_path(std::path::Path::new(&space.directory)),
                alias.as_bytes(),
            )?;
        }
        std::fs::create_dir_all(self.get_change_path(alias)).ok();
        let f = std::fs::File::create(self.get_change_metadata_path(alias))?;
        let mut buf = std::io::BufWriter::new(f);
        space.encode(&mut buf)?;
        Ok(())
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

    pub fn set_identity(&self, host: &str, token: &str) -> std::io::Result<()> {
        std::fs::write(self.root.join("identity").join(host), token.as_bytes())?;
        Ok(())
    }

    pub fn get_identity(&self, host: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join("identity").join(host)).ok()
    }

    pub fn clear_identity(&self, host: &str) {
        std::fs::remove_file(self.root.join("identity").join(host)).ok();
    }

    pub(crate) async fn validate_basis(&self, basis: service::Basis) -> std::io::Result<u64> {
        if basis.host.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "must provide host name",
            ));
        }

        if basis.owner.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "must provide owner",
            ));
        }

        if basis.name.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "must provide repository name",
            ));
        }

        if basis.change != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "change basis isn't supported yet",
            ));
        }

        let client = self.get_client(&basis.host)?;
        let resp = client
            .get_repository(service::GetRepositoryRequest {
                token: String::new(),
                owner: basis.owner.clone(),
                name: basis.name.clone(),
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
                std::io::ErrorKind::NotFound,
                format!("failed to read repository: {}", resp.error_message),
            ));
        }

        Ok(if basis.index == 0 {
            resp.index
        } else {
            std::cmp::min(resp.index, basis.index)
        })
    }

    pub(crate) fn set_mtime(&self, path: &std::path::Path, mtime: u64) -> std::io::Result<()> {
        let p =
            std::ffi::CString::new(path.as_os_str().as_bytes()).expect("failed to create cstring");
        let times = [
            libc::timeval {
                tv_sec: mtime as libc::time_t,
                tv_usec: 0,
            },
            libc::timeval {
                tv_sec: mtime as libc::time_t,
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
}

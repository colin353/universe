use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;

use bus::Deserialize;
use std::io::Write;

fn mtime(m: &std::fs::Metadata) -> u64 {
    let mt = match m.modified() {
        Ok(m) => m,
        Err(_) => return 0,
    };
    let since_epoch = mt.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as u64
}

pub fn metadata_compatible(file: service::FileView, m: &std::fs::Metadata) -> bool {
    if !file.get_is_dir() && file.get_length() != m.len() {
        return false;
    }

    if mtime(m) == file.get_mtime() {
        return true;
    }
    false
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
        Ok(())
    }

    pub(crate) fn get_blob_path(&self, sha: &[u8]) -> std::path::PathBuf {
        self.root.join("blobs").join(core::fmt_sha(sha))
    }

    pub(crate) fn get_blob(&self, sha: &[u8]) -> Option<Vec<u8>> {
        std::fs::read(self.get_blob_path(sha)).ok()
    }

    pub(crate) fn get_change_path(&self, alias: &str) -> std::path::PathBuf {
        self.root.join("changes").join("by_alias").join(alias)
    }

    pub(crate) fn get_change_dir_path(&self, dir: &std::path::Path) -> std::path::PathBuf {
        let hash = core::fmt_sha(&core::hash_bytes(dir.as_os_str().as_bytes()));
        self.root.join("changes").join("by_dir").join(hash)
    }

    pub(crate) fn get_change_by_alias(&self, alias: &str) -> Option<service::Change> {
        let bytes = std::fs::read(self.get_change_path(alias)).ok()?;
        Some(service::Change::decode(&bytes).ok()?)
    }

    pub(crate) fn find_unused_alias(&self, original: &str) -> String {
        let mut idx = 1;
        let mut alias = original.to_string();
        while self.get_change_path(&alias).exists() {
            alias = format!("{}-{}", original, idx);
            idx += 1;
        }
        alias
    }

    pub(crate) fn get_change_by_dir(&self, dir: &std::path::Path) -> Option<service::Change> {
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

    pub(crate) fn validate_basis(&self, basis: service::BasisView) -> std::io::Result<u64> {
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

    pub(crate) fn write_dir(
        &self,
        path: &std::path::Path,
        file: service::FileView,
    ) -> std::io::Result<()> {
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

    pub(crate) fn write_file(
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
}

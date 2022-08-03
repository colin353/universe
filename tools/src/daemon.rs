use std::collections::HashSet;

struct SrcDaemon {
    table: managed_largetable::ManagedLargeTable,
    root: std::path::PathBuf,
}

impl SrcDaemon {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            table: managed_largetable::ManagedLargeTable::new(root.join("db"))?,
            root,
        })
    }

    pub fn create(&self, owner: &str, name: &str, alias: &str) -> std::io::Result<()> {
        self.table.write(
            "repos".to_string(),
            format!("{}/{}", owner, name),
            0,
            service::Repository {
                host: String::new(),
                owner: owner.to_string(),
                name: name.to_string(),
                alias: alias.to_string(),
            },
        )?;
        Ok(())
    }

    pub fn link(&self, host: &str, owner: &str, name: &str, alias: &str) -> std::io::Result<()> {
        self.table.write(
            "repos".to_string(),
            format!("{}/{}", owner, name),
            0,
            service::Repository {
                host: host.to_string(),
                owner: owner.to_string(),
                name: name.to_string(),
                alias: alias.to_string(),
            },
        )?;
        self.table.write(
            "aliases".to_string(),
            alias.to_string(),
            0,
            format!("{}/{}/{}", host, owner, name),
        )?;

        Ok(())
    }

    pub fn new_branch(
        &self,
        repo: String,
        branch_name: String,
        directory: String,
    ) -> std::io::Result<()> {
        let repository: service::Repository =
            self.table.read("repos", &repo, 0).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("repository {} doesn't exist", repo),
                )
            })??;

        let index = if repository.host.is_empty() {
            1
        } else {
            todo!("I don't know how to look up the latest change from an external repo yet!")
        };

        self.table.write(
            "branches".to_string(),
            repo.clone(),
            0,
            service::Branch {
                repository: repo,
                basis: service::Basis {
                    host: repository.host,
                    owner: repository.owner,
                    name: repository.name,
                    index,
                },
                directory: directory,
            },
        )?;

        Ok(())
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

    pub fn apply_diff(
        &self,
        repo: String,
        diffs: Vec<service::FileDiff>,
        index: u64,
        basis: service::BasisView,
    ) -> std::io::Result<()> {
        let mtime = managed_largetable::timestamp_usec() / 1_000_000;
        let mut modified_paths = HashSet::new();

        for diff in &diffs {
            modified_paths.insert(&diff.path);
            if diff.kind == service::DiffKind::Removed {
                // Delete it
                self.table.delete(
                    format!("code/submitted/{}", repo),
                    format!("{}/{}", diff.path.split("/").count(), diff.path),
                    index,
                )?;
                continue;
            }

            if diff.is_dir {
                self.table.write(
                    format!("code/submitted/{}", repo),
                    format!("{}/{}", diff.path.split("/").count(), diff.path),
                    index,
                    service::File {
                        is_dir: true,
                        mtime,
                        sha: vec![],
                    },
                )?;
                continue;
            }

            // Figure out what the byte content of the file is from the diff
            let content: Vec<u8> = if diff.kind == service::DiffKind::Added {
                diff.differences[0].data.clone()
            } else {
                let original = self.get_blob_from_path(basis, &diff.path)?;

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
                self.table.write(
                    "code/blobs".to_string(),
                    sha_str,
                    0,
                    bus::PackedOut(&content),
                )?;
            }

            self.table.write(
                format!("code/submitted/{}", repo),
                format!("{}/{}", diff.path.split("/").count(), diff.path),
                index,
                service::File {
                    is_dir: false,
                    mtime,
                    sha: sha.into(),
                },
            )?;
        }

        let mut modified_parents = HashSet::new();
        for path in &modified_paths {
            for (idx, _) in path.rmatch_indices("/") {
                modified_parents.insert(&path[0..idx]);
            }
        }

        // Touch all parent folders to update their mtime
        for path in modified_parents {
            self.table.write(
                format!("code/submitted/{}", repo),
                format!("{}/{}", path.split("/").count(), path),
                index,
                service::File {
                    is_dir: true,
                    mtime,
                    sha: vec![],
                },
            )?;
        }

        Ok(())
    }

    pub fn setup_dir(&self) -> u8 {
        3
    }
}

pub fn main() {}

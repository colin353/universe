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

    pub fn apply_diff(&self, repo: String, diffs: Vec<service::FileDiff>) -> std::io::Result<()> {
        let mtime = managed_largetable::timestamp_usec() / 1_000_000;
        for diff in diffs {
            if diff.kind == service::DiffKind::Removed {
                // Delete it
            }

            // Figure out what the byte content of the file is from the diff
            let content: Vec<u8> = if diff.kind == service::DiffKind::Added {
                diff.differences[0].data.clone()
            } else {
                Vec::new()
            };

            // Write to the blobs table if no blob is present
            let sha = vec![0];

            self.table.write(
                format!("code/submitted/{}", repo),
                format!("{}/{}", diff.path.split("/").count(), diff.path),
                mtime,
                service::File {
                    is_dir: diff.is_dir,
                    mtime,
                    sha,
                },
            )?;
        }

        Ok(())
    }
}

pub fn main() {}

use std::collections::{HashMap, HashSet};

fn decode_key<'a>(key: &'a str) -> std::io::Result<(usize, &'a str)> {
    match key.find('/') {
        Some(idx) => {
            let depth = key[0..idx].parse::<usize>().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "metadata path did not contain numeric leading depth!",
                )
            })?;
            Ok((depth, &key[idx + 1..]))
        }
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid metadata path",
            ))
        }
    }
}

impl crate::Src {
    pub async fn checkout(
        &self,
        dir: std::path::PathBuf,
        basis: service::Basis,
    ) -> std::io::Result<std::path::PathBuf> {
        let mut directory = dir;
        let mut existing_space = None;
        let mut existing_alias = String::new();

        // Phase 0: Preflight checks
        //      If the current directory already corresponds to an attached space, snapshot that
        //      space before checkout.
        if let Some(alias) = self.get_change_alias_by_dir(&directory) {
            let mut space = match self.get_change_by_alias(&alias) {
                Some(c) => c,
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "existing space is in an invalid state",
                    ));
                }
            };
            directory = std::path::PathBuf::from(&space.directory);
            let resp = self
                .snapshot(service::SnapshotRequest {
                    dir: directory.to_str().unwrap().to_owned(),
                    message: "detached space".to_string(),
                    skip_if_no_changes: true,
                    ..Default::default()
                })
                .await?;
            if resp.failed {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    resp.error_message,
                ));
            }

            existing_space = Some(space);
            existing_alias = alias;
        } else {
            // Check that the directory is empty.
            if directory
                .read_dir()
                .map(|mut i| i.next().is_some())
                .unwrap_or(false)
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "can't checkout, directory is not empty",
                ));
            }
        }

        println!("phase 0 complete");

        // Phase 1: Prepare for downloading
        //      Iterate differentially through changes and collect a list of blobs for downloading
        let mut shas_to_download: HashSet<Vec<u8>> = HashSet::new();
        let mut file_to_folder_transitions: Vec<(String, bool)> = Vec::new();
        let mut dirs_to_create = Vec::new();
        let mut snapshot_changes = match &existing_space {
            None => HashMap::new(),
            Some(_) => {
                let mut output = HashMap::new();
                if let Some(s) = self.get_latest_snapshot(&existing_alias)? {
                    for file in s.files {
                        output.insert(file.path.clone(), file);
                    }
                }
                output
            }
        };
        let metadata = self.get_metadata(basis.clone()).await?;
        let previous_metadata = match &existing_space {
            Some(space) => self.get_metadata(space.basis.clone()).await?,
            None => crate::metadata::Metadata::empty(),
        };
        for (key, maybe_prev_file, maybe_new_file) in previous_metadata.diff(&metadata) {
            let (depth, path) = decode_key(&key)?;

            if let (Some(prev), Some(new)) = (maybe_prev_file, maybe_new_file) {
                if prev.get_is_dir() != new.get_is_dir() {
                    // Track file <--> folder transitions, because they will need to be
                    // resolved first
                    file_to_folder_transitions.push((path.to_owned(), prev.get_is_dir()));
                } else if snapshot_changes.contains_key(path) {
                    // No need to undo snapshot changes if they will be undone anyway
                    snapshot_changes.remove(path);
                }
            }

            if let Some(file) = maybe_new_file {
                if file.get_is_dir() {
                    dirs_to_create.push((depth, directory.join(path), file.get_mtime()));
                }

                if !self.get_blob_path(file.get_sha()).exists() {
                    shas_to_download.insert(file.get_sha().to_owned());
                }
            }
        }

        println!("phase 1 complete");

        // Phase 2: Download all required blobs
        let client = self.get_client(&basis.host)?;
        let token = self.get_identity(&basis.host).unwrap_or_else(String::new);
        for shas in shas_to_download.into_iter().collect::<Vec<_>>().chunks(250) {
            let resp = match client
                .get_blobs(service::GetBlobsRequest {
                    token: token.clone(),
                    shas: shas.to_owned(),
                })
                .await
            {
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "failed to download blobs",
                    ));
                }
                Ok(r) => r,
            };

            if resp.failed {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("failed to download blobs: {}", resp.error_message),
                ));
            }
            if resp.blobs.len() != shas.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("failed to download blobs: {}", resp.error_message),
                ));
            }

            for blob in &resp.blobs {
                std::fs::write(self.get_blob_path(&blob.sha), &blob.data)?;
            }
        }

        println!("phase 2 complete");

        // Phase 3a: Detach the space if one existed
        if let Some(mut space) = existing_space {
            std::fs::remove_file(self.get_change_dir_path(std::path::Path::new(&space.directory)))
                .ok();
            space.directory = String::new();
            self.set_change_by_alias(&existing_alias, &space)?;
        }

        println!("phase 3a complete");

        // Phase 3b: Remove all file <--> directory transitions
        for (path, is_dir) in file_to_folder_transitions {
            if is_dir {
                std::fs::remove_dir(directory.join(path))?;
            } else {
                std::fs::remove_file(directory.join(path))?;
            }
        }

        println!("phase 3b complete");

        // Phase 3c: Create all required directories
        dirs_to_create.sort_by_key(|(depth, _, _)| *depth);
        for (_, path, _) in &dirs_to_create {
            std::fs::create_dir(path).ok();
        }

        println!("phase 3c complete");

        // Phase 4: Create all files
        //      Iterate differentially a second time, and create all files by copying from blob
        //      storage. Delete all unnecessary files and folders.
        let mut dirs_to_remove = Vec::new();
        for (key, maybe_prev_file, maybe_new_file) in previous_metadata.diff(&metadata) {
            let (depth, path) = decode_key(&key)?;

            match (maybe_prev_file, maybe_new_file) {
                // If there's a new file to write...
                (_, Some(new)) => {
                    // Directories are already created on the first pass
                    if !new.get_is_dir() {
                        let dest = directory.join(path);
                        std::fs::copy(self.get_blob_path(new.get_sha()), &dest)?;
                        self.set_mtime(&dest, new.get_mtime())?;
                    }
                }
                // If there's an old file to remove...
                (Some(prev), None) => {
                    if prev.get_is_dir() {
                        dirs_to_remove.push(directory.join(path));
                    } else {
                        std::fs::remove_file(directory.join(path))?;
                    }
                }
                // Shouldn't be possible
                (None, None) => (),
            }
        }

        println!("phase 4 complete");

        // Phase 5: Revert all remaining snapshot changes
        for (path, file) in snapshot_changes {
            match file.kind {
                service::DiffKind::Removed => {
                    // It was removed. If it exists in the new metadata, restore it.
                    if let Some(file) = metadata.get(&path) {
                        std::fs::copy(self.get_blob_path(file.get_sha()), &path)?;
                        self.set_mtime(std::path::Path::new(&path), file.get_mtime())?;
                    }
                }
                service::DiffKind::Added => {
                    // It was added but shouldn't exist, remove it
                    if file.is_dir {
                        std::fs::remove_dir(directory.join(path))?;
                    } else {
                        std::fs::remove_file(directory.join(path))?;
                    }
                }
                // Should not be possible
                _ => (),
            }
        }

        println!("phase 5 complete");

        // Clean up all directories to delete
        for dir in dirs_to_remove {
            std::fs::remove_dir(dir).ok();
        }

        println!("cleanup complete");

        // Phase 6: Set mtime for directories
        //      The mtime for directories may be modified in phase 4, so we need to go back through
        //      a final pass and set the mtime for all directories.
        for (_, path, mtime) in &dirs_to_create {
            self.set_mtime(path, *mtime)?;
        }

        println!("phase 6 complete");

        Ok(directory)
    }

    pub async fn apply_snapshot(
        &self,
        dir: &std::path::Path,
        basis: service::Basis,
        differences: &[service::FileDiff],
    ) -> std::io::Result<()> {
        let metadata = self.get_metadata(basis).await?;
        for diff in differences {
            match diff.kind {
                service::DiffKind::Added => {
                    let data = core::apply(diff.as_view(), &[])?;
                    std::fs::write(&diff.path, data)?;
                }
                service::DiffKind::Modified => {
                    let original = match metadata.get(&diff.path) {
                        Some(f) => std::fs::read(self.get_blob_path(f.get_sha()))?,
                        None => Vec::new(),
                    };
                    let data = core::apply(diff.as_view(), &original)?;
                    std::fs::write(&diff.path, data)?;
                }
                service::DiffKind::Removed => {
                    std::fs::remove_file(dir.join(&diff.path))?;
                }
                // Should be impossible
                _ => (),
            }
        }
        Ok(())
    }
}

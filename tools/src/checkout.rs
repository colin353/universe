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
    pub(crate) fn __checkout(
        &self,
        dir: std::path::PathBuf,
        basis: service::BasisView,
    ) -> std::io::Result<()> {
        let mut directory = dir;
        let mut existing_space = None;
        let mut existing_alias = String::new();
        let index = self.validate_basis(basis)?;

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
            let resp = self.snapshot(service::SnapshotRequest {
                dir: directory.to_str().unwrap().to_owned(),
                message: "detached space".to_string(),
                skip_if_no_changes: true,
                ..Default::default()
            })?;
            if resp.failed {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    resp.error_message,
                ));
            }

            // Detach the space
            space.directory = String::new();
            self.set_change_by_alias(&alias, &space);

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

        // Phase 1: Prepare for downloading
        //      Iterate differentially through changes and collect a list of blobs for downloading
        let mut dirs_to_delete: HashSet<&str> = HashSet::new();
        let mut shas_to_download: HashSet<Vec<u8>> = HashSet::new();
        let mut snapshot_changes = match &existing_space {
            None => HashMap::new(),
            Some(space) => {
                let mut output = HashMap::new();
                if let Some(s) = self.get_latest_snapshot(&existing_alias)? {
                    for file in s.files {
                        output.insert(file.path.clone(), file);
                    }
                }
                output
            }
        };
        let metadata = self.get_metadata(basis)?;
        let previous_metadata = match existing_space {
            Some(space) => self.get_metadata(space.basis.as_view())?,
            None => crate::metadata::Metadata::empty(),
        };
        for (key, maybe_prev_file, maybe_new_file) in previous_metadata.diff(&metadata) {
            let (depth, path) = decode_key(&key)?;
        }

        Ok(())
    }
}

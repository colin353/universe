use std::ops::DerefMut;
use std::sync::{Arc, Mutex, RwLock};

use crate::{helpers, metadata};

impl crate::Src {
    pub fn diff_from(
        &self,
        root: std::path::PathBuf,
        path: std::path::PathBuf,
        metadata: metadata::Metadata<'static>,
    ) -> std::io::Result<Vec<service::FileDiff>> {
        let metadata = Arc::new(metadata);
        let observed: Arc<Mutex<Vec<std::path::PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
        let differences: Arc<Mutex<Vec<service::FileDiff>>> = Arc::new(Mutex::new(Vec::new()));
        let pool = pool::PoolQueue::new(16);
        let _pool = pool.clone();
        let _self = self.clone();
        let _root = root.clone();
        let _differences = differences.clone();
        pool.start(move |p| {
            _self.__diff_from_inner(
                &_pool,
                _root.clone(),
                p,
                &metadata,
                &_differences,
                &observed,
            );
        });
        pool.enqueue(path);

        // Join, check observed vs. expected
        pool.join();

        let out = { std::mem::replace(differences.lock().unwrap().deref_mut(), Vec::new()) };

        Ok(out)
    }

    fn __diff_from_inner(
        &self,
        pool: &pool::PoolQueue<std::path::PathBuf>,
        root: std::path::PathBuf,
        path: std::path::PathBuf,
        metadata: &metadata::Metadata,
        differences: &Mutex<Vec<service::FileDiff>>,
        observed: &Mutex<Vec<std::path::PathBuf>>,
    ) -> std::io::Result<()> {
        let get_metadata =
            |p: &std::path::Path| -> Option<service::FileView> { metadata.get(p.to_str()?) };

        let mut observed_paths = Vec::new();
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let ty = entry.file_type()?;

            if ty.is_symlink() {
                continue;
            }

            let path = entry.path();
            observed_paths.push(path.clone());

            let relative_path = path
                .strip_prefix(&root)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;

            // Entry is a directory. We must always recurse directories
            if ty.is_dir() {
                if get_metadata(&relative_path).is_none() {
                    differences.lock().unwrap().push(service::FileDiff {
                        path: relative_path
                            .to_str()
                            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                            .to_string(),
                        differences: vec![],
                        is_dir: true,
                        kind: service::DiffKind::Added,
                    });
                }
                pool.enqueue(path);
                continue;
            }

            // Entry is a file.
            if let Some(s) = get_metadata(&relative_path) {
                let metadata = entry.metadata()?;
                if helpers::metadata_compatible(s, &metadata) {
                    continue;
                }

                let modified = std::fs::read(&path)?;
                if core::hash_bytes(&modified) == s.get_sha() {
                    continue;
                }

                let original = match self.get_blob(s.get_sha()) {
                    Some(o) => o,
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("blob {:?} not found!", core::fmt_sha(s.get_sha())),
                        ));
                    }
                };

                differences.lock().unwrap().push(service::FileDiff {
                    path: relative_path
                        .to_str()
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_string(),
                    differences: core::diff(&original, &modified),
                    is_dir: false,
                    kind: service::DiffKind::Modified,
                });
            } else {
                let mut data = Vec::new();
                core::compress_rw(
                    &mut std::io::BufReader::new(std::fs::File::open(&path)?),
                    &mut data,
                )?;

                differences.lock().unwrap().push(service::FileDiff {
                    path: relative_path
                        .to_str()
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                        .to_string(),
                    differences: vec![service::ByteDiff {
                        start: 0,
                        end: 0,
                        kind: service::DiffKind::Added,
                        data,
                        compression: service::CompressionKind::LZ4,
                    }],
                    is_dir: false,
                    kind: service::DiffKind::Added,
                });
            }
        }

        observed_paths.sort();

        let nothing: Vec<std::path::PathBuf> = Vec::new();
        let mut observed_iter = observed_paths.iter().peekable();

        let relative_path = path
            .strip_prefix(&root)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        let filter = metadata.filter_key(relative_path.to_str().unwrap());
        let mut expected_iter = metadata
            .list_directory(&filter)
            .map(|(p, _)| root.join(p))
            .peekable();

        loop {
            match (expected_iter.peek(), observed_iter.peek()) {
                (Some(exp), Some(obs)) => {
                    if &exp == obs {
                        expected_iter.next();
                        observed_iter.next();
                        continue;
                    }

                    if obs > &exp {
                        // We missed an expected document. Report it as missing
                        differences.lock().unwrap().push(service::FileDiff {
                            path: exp
                                .strip_prefix(&root)
                                .map_err(|_| {
                                    std::io::Error::from(std::io::ErrorKind::InvalidInput)
                                })?
                                .to_str()
                                .ok_or_else(|| {
                                    std::io::Error::from(std::io::ErrorKind::InvalidInput)
                                })?
                                .to_string(),
                            differences: vec![],
                            is_dir: false,
                            kind: service::DiffKind::Removed,
                        });

                        expected_iter.next();
                    } else {
                        // We got an extra document, this case is already covered
                        observed_iter.next();
                    }
                }
                (Some(exp), None) => {
                    // We missed an expected document. Report it as missing
                    differences.lock().unwrap().push(service::FileDiff {
                        path: exp
                            .strip_prefix(&root)
                            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                            .to_str()
                            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                            .to_string(),
                        differences: vec![],
                        is_dir: false,
                        kind: service::DiffKind::Removed,
                    });

                    expected_iter.next();
                }
                _ => break,
            }
        }

        Ok(())
    }
}

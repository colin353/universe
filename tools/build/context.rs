use crate::core::{BuildActions, Context};

use std::collections::HashMap;
use std::sync::Arc;

impl Context {
    pub fn new(cache_dir: std::path::PathBuf) -> Self {
        Self {
            actions: BuildActions::new(),
            lockfile: Arc::new(HashMap::new()),
            start_time: std::time::Instant::now(),
            cache_dir,
            target: None,
            target_hash: None,
        }
    }

    pub fn with_target(&self, target: &str) -> Self {
        let mut s = self.clone();
        s.target = Some(target.to_string());
        s
    }

    pub fn with_config(&self, config: &str) -> Self {
        let mut s = self.clone();
        s
    }

    pub fn get_locked_version(&self, target: &str) -> std::io::Result<String> {
        self.lockfile
            .get(target)
            .map(|s| s.to_string())
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{target} does not have a lockfile entry!"),
            ))
    }

    pub fn scratch_dir(&self) -> std::path::PathBuf {
        match (self.target.as_ref(), self.target_hash.as_ref()) {
            (Some(t), None) => {
                let v = self
                    .get_locked_version(&t)
                    .unwrap_or_else(|_| String::new());
                self.cache_dir
                    .join("resolve")
                    .join("scratch")
                    .join(format!("{}-{v}", to_dir(t)))
            }
            (Some(t), Some(h)) => self
                .cache_dir
                .join("build")
                .join("scratch")
                .join(format!("{}-{h}", to_dir(t))),
            (None, None) => self.cache_dir.clone(),
            _ => panic!("must have attached target if hash is present!"),
        }
    }

    pub fn working_directory(&self) -> std::path::PathBuf {
        match (self.target.as_ref(), self.target_hash.as_ref()) {
            (Some(t), None) => {
                let v = self
                    .get_locked_version(&t)
                    .unwrap_or_else(|_| String::new());
                self.cache_dir
                    .join("resolve")
                    .join(format!("{}-{v}", to_dir(t)))
            }
            (Some(t), Some(h)) => self
                .cache_dir
                .join("build")
                .join(format!("{}-{h}", to_dir(t))),
            (None, None) => self.cache_dir.clone(),
            _ => panic!("must have attached target if hash is present!"),
        }
    }
}

fn to_dir(name: &str) -> String {
    name.replace(&[':', '/', '@'], "_")
}

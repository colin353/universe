use crate::{BuildHash, Error};

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub trait FileResolver {
    fn get_hash(&self, path: &str) -> Result<BuildHash, Error>;
}

pub struct FakeFileResolver {
    files: HashMap<String, String>,
}

impl FakeFileResolver {
    pub fn new(files: Vec<(&str, &str)>) -> Self {
        Self {
            files: files
                .into_iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        }
    }
}

impl FileResolver for FakeFileResolver {
    fn get_hash(&self, path: &str) -> Result<BuildHash, Error> {
        let content = match self.files.get(path) {
            Some(f) => f,
            None => return Err(Error::new(format!("unable to resolve file {}", path))),
        };

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(hasher.finish())
    }
}

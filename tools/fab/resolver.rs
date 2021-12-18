use crate::{BuildHash, Error};

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub trait Resolver {
    // Get the content of the file
    fn get_content(&self, origin: &str, path: &str) -> Result<String, Error>;

    // Put the file at the destination
    fn realize_at(&self, origin: &str, path: &str, dest: &std::path::Path) -> Result<(), Error>;

    // Get the hash of the file
    fn get_hash(&self, origin: &str, path: &str) -> Result<BuildHash, Error> {
        let content = self.get_content(origin, path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(hasher.finish())
    }
}

pub struct FakeResolver {
    files: HashMap<String, String>,
}

impl Resolver for FakeResolver {
    fn get_content(&self, origin: &str, path: &str) -> Result<String, Error> {
        if !origin.is_empty() {
            return Err(Error::new(format!(
                "I don't know how to resolve origin `{}`",
                origin
            )));
        }

        match self.files.get(path) {
            Some(f) => Ok(f.to_string()),
            None => Err(Error::new(format!("unable to resolve file {}", path))),
        }
    }

    fn realize_at(&self, origin: &str, path: &str, dest: &std::path::Path) -> Result<(), Error> {
        if !origin.is_empty() {
            return Err(Error::new(format!(
                "I don't know how to resolve origin `{}`",
                origin
            )));
        }

        let content = match self.files.get(path) {
            Some(f) => f,
            None => return Err(Error::new(format!("unable to resolve file {}", path))),
        };
        if let Err(e) = std::fs::write(dest, content) {
            return Err(Error::new(format!(
                "unable to realize file {} at destination {}: {:?}",
                path,
                dest.display(),
                e
            )));
        }
        Ok(())
    }
}

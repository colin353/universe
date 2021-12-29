use crate::resolver::Resolver;
use crate::Error;

pub struct FilesystemResolver {}

impl FilesystemResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Resolver for FilesystemResolver {
    fn get_content(&self, origin: &str, path: &str) -> Result<String, Error> {
        if !origin.is_empty() {
            return Err(Error::new("I don't know how to resolve non-empty origins!"));
        }
        match std::fs::read_to_string(path) {
            Ok(c) => Ok(c),
            Err(e) => Err(Error::new(format!("Failed to resolve {}: {:?}", path, e))),
        }
    }

    fn realize_at(&self, origin: &str, path: &str, dest: &std::path::Path) -> Result<(), Error> {
        if !origin.is_empty() {
            return Err(Error::new("I don't know how to resolve non-empty origins!"));
        }
        match std::fs::copy(path, dest) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::new(format!(
                "Failed to realize {} to {:?}: {:?}",
                path, dest, e
            ))),
        }
    }
}

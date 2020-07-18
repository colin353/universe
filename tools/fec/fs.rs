use std::io::{Error, ErrorKind, Result, Write};
use std::path::Path;

use std::collections::HashMap;

pub struct FSAccessor {
    fake: bool,
    filesystem: HashMap<String, String>,
}

impl FSAccessor {
    pub fn new() -> Self {
        Self {
            fake: false,
            filesystem: HashMap::new(),
        }
    }

    pub fn new_fake() -> Self {
        Self {
            fake: true,
            filesystem: HashMap::new(),
        }
    }

    pub fn read_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        if !self.fake {
            return std::fs::read_to_string(path);
        }

        if let Some(f) = self.filesystem.get(path.as_ref().to_str().unwrap()) {
            Ok(f.clone())
        } else {
            Err(Error::new(ErrorKind::NotFound, "not found"))
        }
    }

    pub fn write_string<P: AsRef<Path>>(&mut self, path: P, data: &str) -> Result<()> {
        if !self.fake {
            if let Some(p) = path.as_ref().parent() {
                std::fs::create_dir_all(p)?;
            }

            let mut f = std::fs::File::create(path)?;
            f.write(data.as_bytes())?;
            return Ok(());
        }

        self.filesystem
            .insert(path.as_ref().to_str().unwrap().to_owned(), data.to_owned());
        Ok(())
    }
}

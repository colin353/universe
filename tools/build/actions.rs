use crate::core::{BuildActions, BuildResult};

impl BuildActions {
    pub fn new() -> Self {
        Self {}
    }

    pub fn download<S: Into<String>, P: Into<std::path::PathBuf>>(
        &self,
        url: S,
        dest: P,
    ) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "download isn't implemented (tried to download from {:?} to {:?})!",
                url.into(),
                dest.into()
            ),
        ))
    }
}

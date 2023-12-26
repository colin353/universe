use crate::core::{Config, Context, ResolverPlugin};

#[derive(Debug)]
pub struct CargoResolver {}

impl CargoResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ResolverPlugin for CargoResolver {
    fn can_resolve(&self, target: &str) -> bool {
        target.starts_with("cargo://")
    }

    fn resolve(&self, context: Context, target: &str) -> std::io::Result<Config> {
        let crate_name = target.strip_prefix("cargo://").ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "invalid target name",
        ))?;
        let crate_version = context.get_locked_version(target)?;

        let dest = context.working_directory().join("crate");

        context.actions.download(
            format!(
                "https://crates.io/api/v1/crates/{}/{}/download",
                crate_name, crate_version
            ),
            dest,
        )?;

        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }
}

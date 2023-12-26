use crate::core::{Config, Context, ResolverPlugin};
use crate::plugins::PluginKind;

#[derive(Debug)]
pub struct CargoResolver {}

impl CargoResolver {
    pub fn new() -> Self {
        Self {}
    }
}

fn get_rust_files(
    path: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(&path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_symlink() {
            continue;
        }

        if metadata.is_dir() {
            get_rust_files(&entry.path(), out)?;
        }

        if let Some(ext) = entry.path().extension() {
            if ext == "rs" {
                out.push(entry.path());
            }
        }
    }
    Ok(())
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

        let workdir = context.working_directory();
        std::fs::create_dir_all(&workdir).ok();

        // Download the crate tarball
        let tar_dest = workdir.join("crate.tar");
        context.actions.download(
            &context,
            format!(
                "https://crates.io/api/v1/crates/{}/{}/download",
                crate_name, crate_version
            ),
            &tar_dest,
        )?;

        // Untar the crate tarball
        let dest = workdir.join("crate");
        std::fs::create_dir_all(&dest).ok();
        context.actions.run_process(
            &context,
            "tar",
            &[
                "xzvf",
                &tar_dest.to_string_lossy(),
                "-C",
                &dest.to_string_lossy(),
                "--strip-components=1",
            ],
        )?;

        let mut rust_files = Vec::new();
        get_rust_files(&dest.join("src"), &mut rust_files)?;

        let mut deps = Vec::new();
        if target == "cargo://rand" {
            deps.push("cargo://rand_core".to_string());
        }

        Ok(Config {
            dependencies: deps,
            build_plugin: "@rust_plugin".to_string(),
            location: None,
            sources: rust_files
                .into_iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect(),
            build_dependencies: vec!["@rust_compiler".to_string()],
            kind: PluginKind::RustLibrary.to_string(),
        })
    }
}

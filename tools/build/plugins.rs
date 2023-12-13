use std::collections::HashMap;

use crate::core::*;

pub mod PluginKind {
    pub const RustLibrary: &str = "rust_library";
    pub const RustBinary: &str = "rust_binary";
}

#[derive(Debug)]
pub struct RustPlugin {}

impl RustPlugin {
    fn build_library(&self, config: Config, deps: HashMap<String, BuildOutput>) -> BuildResult {
        let compiler = match config
            .build_dependencies
            .get(0)
            .and_then(|t| deps.get(t))
            .and_then(|f| f.outputs.get(0))
        {
            Some(t) => t,
            None => {
                return BuildResult::Failure(
                    "the rust compiler must be specified as a build_dependency!".to_string(),
                )
            }
        };

        let out_file = "/tmp/liblhello.rlib";

        let out = match std::process::Command::new(compiler)
            .args(config.sources)
            .args([
                "--crate-type",
                "rlib",
                "--crate-name",
                "lhello",
                "-o",
                out_file,
            ])
            .output()
        {
            Ok(o) => o,
            Err(e) => return BuildResult::Failure(format!("failed to invoke compiler: {e:?}")),
        };

        if !out.status.success() {
            return BuildResult::Failure(format!(
                "compilation failed: {:#?}{:#?}",
                std::str::from_utf8(&out.stdout).unwrap_or_default(),
                std::str::from_utf8(&out.stderr).unwrap_or_default(),
            ));
        }

        BuildResult::Success(BuildOutput {
            outputs: vec![std::path::PathBuf::from(out_file)],
        })
    }

    fn build_binary(&self, config: Config, deps: HashMap<String, BuildOutput>) -> BuildResult {
        let compiler = match config
            .build_dependencies
            .get(0)
            .and_then(|t| deps.get(t))
            .and_then(|f| f.outputs.get(0))
        {
            Some(t) => t,
            None => {
                return BuildResult::Failure(
                    "the rust compiler must be specified as a build_dependency!".to_string(),
                )
            }
        };

        let out_file = "/tmp/a.out";

        let libraries = config
            .dependencies
            .iter()
            .map(|t| {
                deps.get(t)
                    .expect("dependencies must be built by now!")
                    .outputs
                    .iter()
            })
            .flatten()
            .map(|s| {
                vec![
                    "--extern".to_string(),
                    format!("lhello={}", s.as_path().display().to_string()),
                ]
                .into_iter()
            })
            .flatten();

        let mut cmd = std::process::Command::new(compiler);
        cmd.args(config.sources)
            .args(libraries)
            .args(["-o", out_file]);

        let out = match cmd.output() {
            Ok(o) => o,
            Err(e) => return BuildResult::Failure(format!("failed to invoke compiler: {e:?}")),
        };

        if !out.status.success() {
            return BuildResult::Failure(format!(
                "compilation failed: {:#?}{:#?}",
                std::str::from_utf8(&out.stdout).unwrap_or_default(),
                std::str::from_utf8(&out.stderr).unwrap_or_default(),
            ));
        }

        BuildResult::Success(BuildOutput {
            outputs: vec![std::path::PathBuf::from(out_file)],
        })
    }
}

impl BuildPlugin for RustPlugin {
    fn build(&self, task: Task, deps: HashMap<String, BuildOutput>) -> BuildResult {
        let config = task.config.expect("config must be specified by now");
        if config.kind == PluginKind::RustLibrary {
            self.build_library(config, deps)
        } else if config.kind == PluginKind::RustBinary {
            self.build_binary(config, deps)
        } else {
            BuildResult::Failure(format!("unsupported target kind: {:?}", config.kind))
        }
    }
}

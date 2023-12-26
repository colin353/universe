use std::collections::HashMap;

use crate::core::*;

pub mod PluginKind {
    pub const RustLibrary: &str = "rust_library";
    pub const RustBinary: &str = "rust_binary";
}

#[derive(Debug)]
pub struct RustPlugin {}

impl RustPlugin {
    fn build_library(
        &self,
        context: &Context,
        name: &str,
        config: Config,
        deps: HashMap<String, BuildOutput>,
    ) -> BuildResult {
        println!("config: {config:#?}");

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

        let working_directory = context.working_directory();
        std::fs::create_dir_all(&working_directory).ok();
        let out_file = working_directory.join(format!("{name}.rlib"));

        let mut root_source_candidates: Vec<_> = config
            .sources
            .iter()
            .filter(|s| s.ends_with("lib.rs") || s.ends_with(&format!("{name}.rs")))
            .collect();
        root_source_candidates.sort_by_key(|c| c.split('/').count());
        let root_source: String = match root_source_candidates.into_iter().next() {
            Some(s) => s.clone(),
            None => {
                return BuildResult::Failure(format!(
                    "no main.rs or {name}.rs source file specified!"
                ))
            }
        };

        let libraries = config
            .dependencies
            .iter()
            .map(|t| {
                deps.get(t)
                    .expect("dependencies must be built by now!")
                    .outputs
                    .iter()
                    .map(move |d| (target_shortname(t).to_string(), d))
            })
            .flatten()
            .map(|(name, s)| {
                vec![
                    "--extern".to_string(),
                    format!("{name}={}", s.as_path().display().to_string()),
                ]
                .into_iter()
            })
            .flatten();

        let mut args: Vec<String> = Vec::new();
        args.push(root_source);
        args.extend(libraries);
        args.extend([
            "--edition=2018".to_string(),
            "--crate-type".to_string(),
            "rlib".to_string(),
            "--crate-name".to_string(),
            name.to_string(),
            "-o".to_string(),
            out_file.to_string_lossy().to_string(),
        ]);

        match context
            .actions
            .run_process(context, compiler, args.as_slice())
        {
            Ok(o) => o,
            Err(e) => return BuildResult::Failure(format!("failed to invoke compiler: {e:?}")),
        };

        BuildResult::Success(BuildOutput {
            outputs: vec![std::path::PathBuf::from(
                out_file.to_string_lossy().to_string(),
            )],
        })
    }

    fn build_binary(
        &self,
        context: &Context,
        name: &str,
        config: Config,
        deps: HashMap<String, BuildOutput>,
    ) -> BuildResult {
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
                    .map(move |d| (target_shortname(t).to_string(), d))
            })
            .flatten()
            .map(|(name, s)| {
                vec![
                    "--extern".to_string(),
                    format!("{name}={}", s.as_path().display().to_string()),
                ]
                .into_iter()
            })
            .flatten();

        let mut root_source_candidates: Vec<_> = config
            .sources
            .iter()
            .filter(|s| s.ends_with("/main.rs") || s.ends_with(&format!("/{name}.rs")))
            .collect();
        root_source_candidates.sort_by_key(|c| c.split('/').count());
        let root_source = match root_source_candidates.iter().next() {
            Some(s) => s,
            None => {
                return BuildResult::Failure(format!(
                    "no main.rs or {name}.rs source file specified!"
                ))
            }
        };

        let mut cmd = std::process::Command::new(compiler);
        cmd.arg(root_source).args(libraries).args(["-o", out_file]);

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
    fn build(
        &self,
        context: Context,
        task: Task,
        deps: HashMap<String, BuildOutput>,
    ) -> BuildResult {
        let name = crate::core::target_shortname(&task.target);

        let config = task.config.expect("config must be specified by now");
        if config.kind == PluginKind::RustLibrary {
            self.build_library(&context, name, config, deps)
        } else if config.kind == PluginKind::RustBinary {
            self.build_binary(&context, name, config, deps)
        } else {
            BuildResult::Failure(format!("unsupported target kind: {:?}", config.kind))
        }
    }
}

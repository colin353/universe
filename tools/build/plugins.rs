use std::collections::HashMap;

use crate::core::*;

#[derive(Debug)]
pub struct RustPlugin {}

impl BuildPlugin for RustPlugin {
    fn build(&self, task: Task, deps: HashMap<String, BuildOutput>) -> BuildResult {
        let config = task.config.expect("config must be specified by now");

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

        let out = match std::process::Command::new(compiler)
            .args(config.sources)
            .args(["-o", out_file])
            .output()
        {
            Ok(o) => o,
            Err(e) => return BuildResult::Failure(format!("failed to invoke compiler: {e:?}")),
        };

        if !out.status.success() {
            return BuildResult::Failure(format!("compilation failed: {:#?}", out.stdout));
        }

        BuildResult::Success(BuildOutput {
            outputs: vec![std::path::PathBuf::from(out_file)],
        })
    }
}

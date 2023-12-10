use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use std::os::unix::fs::PermissionsExt;

use crate::{
    BuildHash, BuildResult, Error, FileResolver, Target, TargetIdentifier, TargetResolver,
};

pub struct ExecutionContext {
    origin: String,
    targets: HashMap<TargetIdentifier, Target>,
    target_resolver: Box<dyn TargetResolver>,
    file_resolver: Box<dyn FileResolver>,
    build_dir: std::path::PathBuf,
}

impl ExecutionContext {
    pub fn new(
        origin: String,
        build_dir: std::path::PathBuf,
        target_resolver: Box<dyn TargetResolver>,
        file_resolver: Box<dyn FileResolver>,
    ) -> Self {
        Self {
            origin,
            targets: HashMap::new(),
            target_resolver,
            file_resolver,
            build_dir,
        }
    }

    pub fn build(&mut self, identifier: &TargetIdentifier) -> Result<BuildResult, Error> {
        // First, resolve all targets and build dependency tree
        let mut to_resolve = HashSet::new();
        to_resolve.insert(identifier.clone());
        while to_resolve.len() > 0 {
            let this_round = std::mem::replace(&mut to_resolve, HashSet::new());
            for target in this_round {
                let resolved = self.target_resolver.resolve(&target)?;

                for dep in &resolved.dependencies() {
                    if !self.targets.contains_key(dep) {
                        to_resolve.insert(dep.clone());
                    }
                }
                self.targets.insert(target, resolved.clone());
            }
        }

        // Second, resolve the build hash
        self.resolve_hash(&identifier)?;

        // Finally, run the build
        self.build_target_and_dependencies(&identifier)
    }

    pub fn resolve_hash(&mut self, identifier: &TargetIdentifier) -> Result<BuildHash, Error> {
        let mut hasher = DefaultHasher::new();
        let dependencies: Vec<TargetIdentifier> = {
            let target = self
                .targets
                .get_mut(identifier)
                .expect("all targets should be resolved");

            // If we already resolved this target through another path, quit early
            if let Some(build_hash) = &target.hash {
                return Ok(*build_hash);
            }

            if target.resolving {
                return Err(Error::new(format!(
                    "circular dependency resolving {:?}",
                    identifier,
                )));
            }
            target.resolving = true;
            target.identifier.hash(&mut hasher);
            target.operation_hash().hash(&mut hasher);

            target.dependencies()
        };

        for dependency in &dependencies {
            self.resolve_hash(dependency)?.hash(&mut hasher);
        }

        let target = self
            .targets
            .get_mut(identifier)
            .expect("all targets should be resolved");

        for file in &target.files {
            self.file_resolver.get_hash(file)?.hash(&mut hasher);
        }

        let build_hash = hasher.finish();
        target.hash = Some(build_hash);
        target.resolving = false;

        Ok(hasher.finish())
    }

    fn build_target_and_dependencies(
        &mut self,
        identifier: &TargetIdentifier,
    ) -> Result<BuildResult, Error> {
        let dependencies: Vec<TargetIdentifier> = {
            let target = self
                .targets
                .get_mut(identifier)
                .expect("all targets should be resolved");

            if let Some(result) = &target.result {
                return Ok(result.clone());
            }

            if target.resolving {
                return Err(Error::new(format!(
                    "circular dependency building {:?}",
                    identifier,
                )));
            }
            target.resolving = true;
            target.dependencies()
        };

        for dependency in &dependencies {
            self.build_target_and_dependencies(dependency)?;
        }

        let result = self.build_ready_target(identifier)?;

        let target = self
            .targets
            .get_mut(identifier)
            .expect("all targets should be resolved");
        target.result = Some(result.clone());
        target.resolving = false;
        Ok(result)
    }

    fn build_ready_target(&mut self, identifier: &TargetIdentifier) -> Result<BuildResult, Error> {
        let target = self
            .targets
            .get(identifier)
            .expect("all targets should be resolved");

        let target_dir = target.build_dir(&self.build_dir);
        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            return Err(Error::new(format!(
                "unable to create build directory: {:?}",
                e
            )));
        }

        // Copy in all of the necessary files
        let mut sources = Vec::new();
        for file in &target.files {
            let dest = target_dir.join(file);
            self.file_resolver.realize_at(file, &dest)?;
            sources.push(dest.into_os_string().into_string().unwrap());
        }

        // Construct environment vars for the build script
        let mut environment: HashMap<String, String> = HashMap::new();
        environment.insert("SOURCE_FILES".to_string(), sources.join("\n"));
        environment.insert(
            "TARGET_DIR".to_string(),
            target_dir.into_os_string().into_string().unwrap(),
        );
        for input in &target.operation.inputs {
            let ident = TargetIdentifier::from_str(&input.target);
            let dep = self
                .targets
                .get(&ident)
                .expect("all targets should be resolved");
            let hash = dep.hash.expect("build hash must be computed by now");
            environment.insert(
                input.name.to_string(),
                format!(
                    "{}/{}",
                    dep.build_dir(&self.build_dir).to_str().unwrap().to_string(),
                    input.filename
                ),
            );
        }

        // Pass along custom variables
        for variable in &target.operation.variables {
            environment.insert(variable.name.clone(), variable.value.clone());
        }

        if target.operation.get_script().filename.len() > 0 {
            let script_ident = TargetIdentifier::from_str(&target.operation.get_script().target);
            let script_target = self
                .targets
                .get(&script_ident)
                .expect("all targets should be resolved");

            let script_build_dir = script_target.build_dir(&self.build_dir);
            let script_path = script_build_dir.join(target.operation.get_script().get_filename());

            // Mark operation script executable
            let mut perms = std::fs::Permissions::from_mode(0o777);
            std::fs::set_permissions(script_path, perms);

            // Execute the operation script
            let status = std::process::Command::new(format!(
                "{}/{}",
                script_build_dir.to_str().unwrap(),
                target.operation.get_script().filename
            ))
            .env_clear()
            .envs(&environment)
            .status();

            let status = match status {
                Ok(s) => s,
                Err(e) => {
                    return Err(Error::new(format!("failed to start build, {:?}", e)));
                }
            };

            if !status.success() {
                return Err(Error::new(format!("failed to build")));
            }
        }

        eprintln!("building {}", target.fully_qualified_name());
        eprintln!("operation: {:?}", target.operation);
        eprintln!("environment: {:?}", environment);

        Ok(BuildResult {
            build_hash: target.hash.expect("target must have hash by now"),
            outputs: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_resolver::FakeFileResolver;
    use crate::target_resolver::FakeTargetResolver;

    #[test]
    fn test_build() {
        let file_resolver = FakeFileResolver::new(vec![
            ("main.rs", "fn main() { println!(\"cool\") }"),
            ("lib.rs", "// TODO: write lib"),
            ("xyz.rs", "my func"),
            ("rustc.sh", "#!/bin/bash\necho $SOURCE_FILES\n"),
        ]);

        let mut op = build_grpc_rust::Operation::new();
        op.name = String::from("compile");
        op.mut_script().target = String::from("//compiler");
        op.mut_script().filename = String::from("rustc.sh");

        let target_resolver = FakeTargetResolver::new(vec![
            Target::for_test("//util:my_lib", &[], &["main.rs", "lib.rs"], op.clone()),
            Target::for_test("//util:my_bin", &["//util:my_lib"], &["xyz.rs"], op),
            Target::for_test(
                "//compiler",
                &[],
                &["rustc.sh"],
                build_grpc_rust::Operation::new(),
            ),
        ]);

        let mut ctx = ExecutionContext::new(
            String::from(""),
            std::path::PathBuf::from("/tmp/builds"),
            Box::new(target_resolver),
            Box::new(file_resolver),
        );
        ctx.build(&TargetIdentifier::from_str("//util:my_bin"))
            .unwrap();
    }
}

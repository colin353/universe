use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::{
    BuildHash, BuildResult, Error, FileResolver, Target, TargetIdentifier, TargetResolver,
};

pub struct ExecutionContext {
    origin: String,
    targets: HashMap<TargetIdentifier, Target>,
    target_resolver: Box<dyn TargetResolver>,
    file_resolver: Box<dyn FileResolver>,
}

impl ExecutionContext {
    pub fn new(
        origin: String,
        target_resolver: Box<dyn TargetResolver>,
        file_resolver: Box<dyn FileResolver>,
    ) -> Self {
        Self {
            origin,
            targets: HashMap::new(),
            target_resolver,
            file_resolver,
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

                for dep in &resolved.dependencies {
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

            target.dependencies.iter().cloned().collect()
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
            target.dependencies.iter().cloned().collect()
        };

        for dependency in &dependencies {
            self.build_target_and_dependencies(dependency)?;
        }

        self.build_ready_target(identifier)
    }

    fn build_ready_target(&mut self, identifier: &TargetIdentifier) -> Result<BuildResult, Error> {
        let target = self
            .targets
            .get(identifier)
            .expect("all targets should be resolved");

        eprintln!("building {}", target.fully_qualified_name());
        eprintln!("operation: {:?}", target.operation);

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
            ("main.rs", "fn hello() -> u64 { 5 }"),
            ("lib.rs", "// TODO: write lib"),
            ("xyz.rs", "my func"),
        ]);
        let target_resolver = FakeTargetResolver::new(vec![
            Target::for_test("//util:my_lib", &[], &["main.rs", "lib.rs"]),
            Target::for_test("//util:my_bin", &["//util:my_lib"], &["xyz.rs"]),
        ]);

        let mut ctx = ExecutionContext::new(
            String::from(""),
            Box::new(target_resolver),
            Box::new(file_resolver),
        );
        let result = ctx
            .build(&TargetIdentifier::from_str("//util:my_bin"))
            .unwrap();

        assert_eq!(result.build_hash, 9900385603230248632);
    }
}

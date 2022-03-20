use pool::PoolQueue;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};

use std::os::unix::fs::PermissionsExt;

use crate::config;
use crate::resolver::Resolver;
use crate::{BuildResult, Error, Target, TargetIdentifier};

pub struct BuildEnvironment<R: Resolver + Sync> {
    inner: Arc<BuildEnvironmentInner<R>>,
}

enum Task {
    Resolve(TargetIdentifier),
    Build(TargetIdentifier),
}

pub struct BuildEnvironmentInner<R: Resolver + Sync> {
    targets: RwLock<HashMap<TargetIdentifier, RwLock<Target>>>,
    rdeps: RwLock<HashMap<TargetIdentifier, Vec<TargetIdentifier>>>,
    asts: RwLock<HashMap<TargetIdentifier, Arc<ccl::AST>>>,
    resolver: R,
    errors: Mutex<Vec<Error>>,
    pool: PoolQueue<Task>,
    root_dir: std::path::PathBuf,
}

impl<R: Resolver + Sync + Send + 'static> BuildEnvironment<R> {
    pub fn new(resolver: R, root_dir: std::path::PathBuf) -> Self {
        let pool = PoolQueue::new(4);

        let inner = Arc::new(BuildEnvironmentInner {
            targets: RwLock::new(HashMap::new()),
            asts: RwLock::new(HashMap::new()),
            rdeps: RwLock::new(HashMap::new()),
            resolver,
            errors: Mutex::new(Vec::new()),
            pool,
            root_dir,
        });

        let _inner = inner.clone();
        inner.pool.start(move |task| match task {
            Task::Resolve(ident) => _inner._resolve_target_worker(ident),
            Task::Build(ident) => _inner._build_target_worker(ident),
        });

        Self {
            inner: inner.clone(),
        }
    }

    pub fn build(&self, identifier: TargetIdentifier) -> Result<BuildResult, Error> {
        // Create relevant directories
        let out_dir = self.inner.root_dir.join("out");
        if let Err(e) = std::fs::create_dir_all(&out_dir) {
            return Err(Error::new(format!(
                "unable to create directory {:?}: {:?}",
                out_dir, e
            )));
        }
        let ops_dir = self.inner.root_dir.join("ops");
        if let Err(e) = std::fs::create_dir_all(&ops_dir) {
            return Err(Error::new(format!(
                "unable to create directory {:?}: {:?}",
                ops_dir, e
            )));
        }
        let tmp_dir = self.inner.root_dir.join("tmp");
        if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
            return Err(Error::new(format!(
                "unable to create directory {:?}: {:?}",
                tmp_dir, e
            )));
        }
        let cache_dir = self.inner.root_dir.join("cache");
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            return Err(Error::new(format!(
                "unable to create directory {:?}: {:?}",
                tmp_dir, e
            )));
        }

        let _inner = self.inner.clone();
        self.inner.pool.enqueue(Task::Resolve(identifier.clone()));

        // Wait for build to complete
        self.inner.pool.join();

        // Check whether it was a failure or not
        {
            let _err = self.inner.errors.lock().unwrap();
            if !_err.is_empty() {
                return Err(Error::from_errors(&_err));
            }
        }

        if let Some(target) = self
            .inner
            .targets
            .read()
            .expect("target must exist by now!")
            .get(&identifier)
        {
            if let Some(res) = &target.read().unwrap().result {
                return Ok(res.clone());
            }
        }

        Err(Error::new("build finished, but produced no result!"))
    }
}

impl<R: Resolver + Sync> BuildEnvironmentInner<R> {
    fn _resolve_target_worker(&self, identifier: TargetIdentifier) {
        if let Err(e) = self.resolve_target(&identifier) {
            self.errors.lock().unwrap().push(e);
        }
    }

    fn _build_target_worker(&self, identifier: TargetIdentifier) {
        if let Err(e) = self.build_target(&identifier) {
            self.errors.lock().unwrap().push(e);
        }
    }

    fn calculate_build_hash(&self, target: &Target) -> Result<crate::BuildHash, Error> {
        let mut build_hash = DefaultHasher::new();
        target.hash(&self.resolver, &mut build_hash);
        for dep in target.dependencies() {
            if let Some(t) = self.targets.read().unwrap().get(&dep) {
                let t = t.read().unwrap();
                if let Some(h) = t.hash {
                    h.hash(&mut build_hash);
                } else {
                    return Err(Error::new(format!(
                        "unable to calculate build hash for {}: dependency {} has no build hash!",
                        target.identifier.fully_qualified_name(),
                        dep.fully_qualified_name(),
                    )));
                }
            } else {
                return Err(Error::new(format!(
                    "unable to calculate build hash for {}: dependency {} is not resolved yet!",
                    target.identifier.fully_qualified_name(),
                    dep.fully_qualified_name(),
                )));
            }
        }
        Ok(build_hash.finish())
    }

    fn resolve_target(&self, identifier: &TargetIdentifier) -> Result<(), Error> {
        // If the target has already been resolved, or is being currently resolved, we're done
        if self.targets.read().unwrap().contains_key(identifier) {
            println!(
                "skip already resolved target {}",
                identifier.fully_qualified_name()
            );
            return Ok(());
        }

        // Write a placeholder target to reserve the slot
        {
            let mut _t = self.targets.write().unwrap();
            if _t.contains_key(identifier) {
                return Ok(());
            }

            let placeholder = Target::new(identifier.clone());
            _t.insert(identifier.clone(), RwLock::new(placeholder));
        }

        // Resolve the build configuration
        let build = self.resolve_config(identifier)?;

        // Extract target definition
        let mut target = config::convert_to_target(build.as_ref(), identifier)?;
        target.resolved = true;

        // Queue dependencies for resolution
        let mut all_built = true;
        let mut all_resolved = true;
        let mut build_hash = DefaultHasher::new();
        target.hash(&self.resolver, &mut build_hash);

        {
            let mut _rdeps = self.rdeps.write().unwrap();
            for dep in target.dependencies() {
                if let Some(t) = self.targets.read().unwrap().get(&dep) {
                    let _target = t.read().unwrap();
                    if _target.result.is_none() {
                        all_built = false;
                    }

                    if _target.resolved {
                        _target.hash.unwrap().hash(&mut build_hash);
                    } else {
                        all_resolved = false;
                    }
                } else {
                    all_built = false;
                    self.pool.enqueue(Task::Resolve(dep.clone()));
                }

                if let Some(r) = _rdeps.get_mut(&dep) {
                    r.push(identifier.clone());
                } else {
                    _rdeps.insert(dep, vec![identifier.clone()]);
                }
            }
        }

        if all_resolved {
            target.hash = Some(build_hash.finish());
        }

        // Save target
        {
            let mut _t = self.targets.write().unwrap();
            _t.insert(identifier.clone(), RwLock::new(target.clone()));
        }

        if all_built {
            self.pool.enqueue(Task::Build(identifier.clone()));
        }

        Ok(())
    }

    fn possibly_build_rdeps(&self, identifier: &TargetIdentifier) {
        if let Some(iter) = self.rdeps.read().unwrap().get(identifier) {
            for rdep in iter {
                let _targets = self.targets.read().unwrap();
                let mut t = _targets.get(&rdep).unwrap().write().unwrap();
                t.built_dependencies += 1;
                if t.built_dependencies == t.dependencies().len() {
                    self.pool.enqueue(Task::Build(t.identifier.clone()));
                };
            }
        }
    }

    fn resolve_config(&self, identifier: &TargetIdentifier) -> Result<Arc<ccl::AST>, Error> {
        let mut build_ident = identifier.clone();
        build_ident.name.clear();
        if let Some(ast) = self.asts.read().unwrap().get(&build_ident) {
            return Ok(ast.clone());
        }

        // Get the content and parse the build file AST
        let build_path = build_ident.build_file();
        let content = self
            .resolver
            .get_content(&build_ident.origin, &build_path)?;

        let ast = match ccl::AST::from_string(content) {
            Ok(ast) => ast,
            Err(e) => {
                return Err(Error::new(format!(
                    "failed to parse BUILD.ccl file {}: {:?}",
                    build_path, e
                )))
            }
        };

        // Store the AST so we don't have to get/parse it again
        let arc_ast = Arc::new(ast);
        self.asts
            .write()
            .unwrap()
            .insert(build_ident, arc_ast.clone());

        Ok(arc_ast)
    }

    fn build_target(&self, identifier: &TargetIdentifier) -> Result<(), Error> {
        let target = self
            .targets
            .read()
            .unwrap()
            .get(identifier)
            .expect("all targets should be resolved")
            .read()
            .unwrap()
            .clone();

        // Don't do anything if we have cached output
        if target.out_dir(&self.root_dir).exists() {
            println!("skipped {}, hit cache", identifier.fully_qualified_name());
            // Build done, store build result
            self.targets
                .read()
                .expect("couldn't lock self.targets!")
                .get(identifier)
                .expect("all targets should be resolved")
                .write()
                .expect("couldn't lock target!")
                .result = Some(BuildResult {
                build_hash: 123,
                outputs: vec![],
            });
            self.possibly_build_rdeps(identifier);
            return Ok(());
        }

        if target.is_operation() {
            // Build done, store build result
            self.targets
                .read()
                .expect("couldn't lock self.targets!")
                .get(identifier)
                .expect("all targets should be resolved")
                .write()
                .expect("couldn't lock target!")
                .result = Some(BuildResult {
                build_hash: 123,
                outputs: vec![],
            });
            self.possibly_build_rdeps(identifier);
            return Ok(());
        }

        // Setup build dir for this target
        self.prepare_for_build(&target)?;

        // Find the operation script
        let op = self
            .targets
            .read()
            .unwrap()
            .get(target.operation.as_ref().unwrap())
            .expect("operation should be resolved!")
            .read()
            .unwrap()
            .clone();

        op.files.iter().next().ok_or_else(|| {
            Error::new(format!(
                "operation {} is missing a script to run!",
                op.identifier.fully_qualified_name()
            ))
        })?;

        self.prepare_for_build(&op)?;

        let mut environment: HashMap<String, String> = HashMap::new();

        for (k, v) in &target.variables {
            environment.insert(k.to_string(), v.to_string());
        }
        environment.insert(
            "CACHE_DIR".to_string(),
            self.root_dir
                .join("cache")
                .into_os_string()
                .into_string()
                .unwrap(),
        );
        environment.insert(
            "OP_DIR".to_string(),
            op.op_dir(&self.root_dir)
                .into_os_string()
                .into_string()
                .unwrap(),
        );
        environment.insert(
            "OUT_DIR".to_string(),
            target
                .tmp_out_dir(&self.root_dir)
                .into_os_string()
                .into_string()
                .unwrap(),
        );
        let srcs = target.build_dir(&self.root_dir).join("src");
        environment.insert(
            "SOURCE_FILES".to_string(),
            target
                .files
                .iter()
                .map(|s| {
                    srcs.join(s.as_str())
                        .into_os_string()
                        .into_string()
                        .unwrap()
                })
                .collect::<Vec<_>>()
                .join("\n"),
        );
        environment.insert(
            "TARGET_NAME".to_string(),
            target.identifier.name.to_string(),
        );
        environment.insert(
            "TARGET_DIR".to_string(),
            target
                .build_dir(&self.root_dir)
                .into_os_string()
                .into_string()
                .unwrap(),
        );

        let status = std::process::Command::new(op.op_script(&self.root_dir))
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

        // Move temp out dir to final out dir
        if let Err(_) = std::fs::rename(
            target.tmp_out_dir(&self.root_dir),
            target.out_dir(&self.root_dir),
        ) {
            return Err(Error::new(format!(
                "failed to rename temporary out dir ({:?}) to final output dir ({:?})!",
                target.tmp_out_dir(&self.root_dir),
                target.out_dir(&self.root_dir),
            )));
        };

        // Build done, store build result
        self.targets
            .read()
            .expect("couldn't lock self.targets!")
            .get(identifier)
            .expect("all targets should be resolved")
            .write()
            .expect("couldn't lock target!")
            .result = Some(BuildResult {
            build_hash: 123,
            outputs: vec![],
        });

        self.possibly_build_rdeps(identifier);

        Ok(())
    }

    // Prepare for build by setting up build/op directory structure
    fn prepare_for_build(&self, target: &Target) -> Result<(), Error> {
        let dest = if target.is_operation() {
            target.op_dir(&self.root_dir)
        } else {
            target.build_dir(&self.root_dir)
        };

        // Clear out the build directory from whatever was previously in it
        if let Err(_) = std::fs::remove_dir_all(&dest) {
            // Ignore failure to remove directory, since that probably measn it doesn't exist
        };
        if let Err(e) = std::fs::create_dir_all(&dest.join("src")) {
            return Err(Error::new(format!(
                "unable to create directory {:?}: {:?}",
                dest, e
            )));
        }

        // Copy all necessary files
        let mut sources = Vec::new();
        for file in &target.files {
            let dest = dest.join("src").join(file);
            self.resolver
                .realize_at(&target.identifier.origin, file, &dest)?;
            sources.push(dest.into_os_string().into_string().unwrap());
        }

        if target.is_operation() {
            // Mark operation script executable
            let perms = std::fs::Permissions::from_mode(0o777);
            if let Err(e) = std::fs::set_permissions(&target.op_script(&self.root_dir), perms) {
                return Err(Error::new(format!(
                    "unable to set script permissions on {:?}: {:?}",
                    &target.op_script(&self.root_dir),
                    e
                )));
            };
        } else {
            if let Err(e) = std::fs::create_dir_all(&target.tmp_out_dir(&self.root_dir)) {
                return Err(Error::new(format!(
                    "unable to create directory {:?}: {:?}",
                    dest, e
                )));
            }
        }

        // Copy all dependency outputs
        for dep in &target.dependencies {
            let _targets = self.targets.read().unwrap();
            let _dep = _targets.get(dep).expect("deps must exist!");
            let dep = _dep.read().unwrap();
            let dest = dest.join(&dep.identifier.origin).join(&dep.identifier.path);
            if let Err(_) = std::fs::create_dir_all(&dest) {
                return Err(Error::new(format!(
                    "unable to create parent directory in {:?}!",
                    dest
                )));
            };
            let dest = dest.join(&dep.identifier.name);
            if let Err(_) = std::os::unix::fs::symlink(dep.out_dir(&self.root_dir), &dest) {
                return Err(Error::new(format!(
                    "unable to create symlink from {:?} to {:?}!",
                    dep.out_dir(&self.root_dir),
                    dest
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::FakeResolver;

    #[test]
    fn test_environment() {
        let resolver = FakeResolver::new(vec![
            (
                "utils/project/BUILD.ccl",
                r#"
            rust_library_operation = {
                srcs = "script.sh"
                deps = [
                    ":rustc"
                ]
            }

            download_rustc = {
                srcs = "download_rustc.sh"
            }

            rustc = {
                srcs = []
                operation = ":download_rustc"
                vars = {
                    URL = "file:///tmp/rustc.tar.gz"
                }
            }

            my_target = {
                srcs = [ "abc.rs" ]
                operation = ":rust_library_operation"
            }
        "#,
            ),
            (
                "utils/project/abc.rs",
                r#"
          fn main() { println!("hello, world!"); }
         "#,
            ),
            (
                "utils/project/script.sh",
                r#"
echo $SOURCE_FILES
"#,
            ),
            (
                "utils/project/download_rustc.sh",
                r#"
echo "downloading $URL..."
            "#,
            ),
        ]);
        let env =
            BuildEnvironment::new(resolver, std::path::PathBuf::from_str("/tmp/fab").unwrap());
        let result = env
            .build(TargetIdentifier::from_str("//utils/project:my_target"))
            .unwrap();

        assert_eq!(result.build_hash, 123);
    }

    // Don't run normally, this does a huge amount of work and
    // downlaods e.g. rustc
    //#[test]
    fn test_rust_environment() {
        let resolver = FakeResolver::new(vec![
            (
                "utils/project/BUILD.ccl",
                r#"
            rust_library_operation = {
                srcs = "script.sh"
                deps = [
                    ":rustc"
                ]
            }

            download_rustc = {
                srcs = "download_rustc.sh"
            }

            rustc = {
                srcs = []
                operation = ":download_rustc"
                vars = {
                    RUSTC_URL = "file:///tmp/rustc.tar.gz"
                    RUST_URL = "file:///tmp/rust.tar.gz"
                }
            }

            my_target = {
                srcs = [ "abc.rs" ]
                operation = ":rust_library_operation"
            }
        "#,
            ),
            (
                "utils/project/abc.rs",
                r#"
          fn main() { println!("hello, world!"); }
         "#,
            ),
            (
                "utils/project/script.sh",
                r#"
$OP_DIR/utils/project/rustc/rustc/bin/rustc \
    --color=always \
    --crate-name=$TARGET_NAME \
    --edition=2018 \
    --codegen=linker=/usr/bin/clang \
    -C link-arg=-fuse-ld=/usr/bin/ld \
    -o $OUT_DIR/$TARGET_NAME \
    -L$OP_DIR/utils/project/rustc/rustc/lib/ \
    -L$OP_DIR/utils/project/rustc/rust-std-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib \
    $SOURCE_FILES
"#,
            ),
            (
                "utils/project/download_rustc.sh",
                r#"
curl $RUSTC_URL -o $OUT_DIR/rustc.tar.gz
tar xzf $OUT_DIR/rustc.tar.gz -C $OUT_DIR --strip-components=1
curl $RUST_URL -o $OUT_DIR/rust.tar.gz
tar xzf $OUT_DIR/rust.tar.gz -C $OUT_DIR --strip-components=1
            "#,
            ),
        ]);
        let env =
            BuildEnvironment::new(resolver, std::path::PathBuf::from_str("/tmp/fab").unwrap());
        let result = env
            .build(TargetIdentifier::from_str("//utils/project:my_target"))
            .unwrap();

        assert_eq!(result.build_hash, 123);
    }
}

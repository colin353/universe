use pool::PoolQueue;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};

use crate::config;
use crate::resolver::Resolver;
use crate::{BuildResult, Error, Target, TargetIdentifier};

pub struct BuildEnvironment<R: Resolver + Sync> {
    inner: Arc<BuildEnvironmentInner<R>>,
}

pub struct BuildEnvironmentInner<R: Resolver + Sync> {
    targets: RwLock<HashMap<TargetIdentifier, RwLock<Target>>>,
    asts: RwLock<HashMap<TargetIdentifier, Arc<ccl::AST>>>,
    resolver: R,
    errors: Mutex<Vec<Error>>,
    pool: PoolQueue<()>,
}

impl<R: Resolver + Sync + Send> BuildEnvironment<R> {
    pub fn new(resolver: R) -> Self {
        let inner = Arc::new(BuildEnvironmentInner {
            targets: RwLock::new(HashMap::new()),
            asts: RwLock::new(HashMap::new()),
            resolver,
            errors: Mutex::new(Vec::new()),
            pool: PoolQueue::new(8),
        });

        Self {
            inner: inner.clone(),
        }
    }

    pub fn build(&self, identifier: TargetIdentifier) -> Result<BuildResult, Error> {
        let _inner = self.inner.clone();
        self.inner.pool.enqueue(());
        self.inner.pool.join();
        Ok(BuildResult {
            build_hash: 000,
            outputs: vec![],
        })
    }
}

impl<R: Resolver + Sync> BuildEnvironmentInner<R> {
    pub fn build(&self, identifier: &TargetIdentifier) -> Result<BuildResult, Error> {
        self.resolve_target(identifier)?;

        Ok(BuildResult {
            build_hash: 123,
            outputs: vec![],
        })
    }

    fn _resolve_target_worker(&self, identifier: TargetIdentifier) {
        if let Err(e) = self.resolve_target(&identifier) {
            self.errors.lock().unwrap().push(e);
        }
    }

    fn resolve_target(&self, identifier: &TargetIdentifier) -> Result<(), Error> {
        // If the target has already been resolved, or is being currently resolved, we're done
        if self.targets.read().unwrap().contains_key(identifier) {
            return Ok(());
        }

        // Write a placeholder target to reserve the slot
        {
            let _t = self.targets.write().unwrap();
            if _t.contains_key(identifier) {
                return Ok(());
            }

            let mut placeholder = Target::new(identifier.clone());
            self.targets
                .write()
                .unwrap()
                .insert(identifier.clone(), RwLock::new(placeholder));
        }

        // Resolve the build file
        let build = self.resolve_build(identifier)?;

        // Extract target definition
        let target = config::convert_to_target(build.as_ref(), identifier)?;

        Ok(())
    }

    fn resolve_build(&self, identifier: &TargetIdentifier) -> Result<Arc<ccl::AST>, Error> {
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
}

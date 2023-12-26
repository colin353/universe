use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Context {
    pub start_time: std::time::Instant,
    pub actions: BuildActions,
    pub lockfile: Arc<HashMap<String, String>>,
    pub cache_dir: std::path::PathBuf,
    pub target: Option<String>,
    pub target_hash: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct BuildActions {}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: usize,
    pub dependencies: Vec<usize>,
    pub target: String,
    pub config: Option<Config>,
    pub result: Option<BuildResult>,
    pub available: bool,
    pub dependencies_ready: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuildResult {
    Success(BuildOutput),
    Failure(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildOutput {
    pub outputs: Vec<std::path::PathBuf>,
}

impl BuildResult {
    pub fn noop() -> Self {
        BuildResult::Success(BuildOutput {
            outputs: Vec::new(),
        })
    }

    pub fn merged<'a, I: Iterator<Item = &'a Self>>(results: I) -> Self {
        let mut outs = Vec::new();
        for result in results {
            match result {
                BuildResult::Success(BuildOutput { outputs }) => {
                    outs.extend(outputs.to_owned());
                }
                _ => return result.clone(),
            }
        }
        BuildResult::Success(BuildOutput { outputs: outs })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub dependencies: Vec<String>,
    pub build_plugin: String,
    pub location: Option<String>,
    pub sources: Vec<String>,
    pub build_dependencies: Vec<String>,
    pub kind: String,
}

impl Config {
    pub fn dependencies(&self) -> Vec<&str> {
        let mut out: Vec<_> = self.dependencies.iter().map(|s| s.as_str()).collect();
        out.push(self.build_plugin.as_str());
        out.extend(self.build_dependencies.iter().map(|s| s.as_str()));
        out
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Resolving,
    Blocked,
    Building,
    Done,
}

impl Task {
    pub fn new(id: usize, target: String) -> Self {
        Self {
            id,
            dependencies: Vec::new(),
            target,
            config: None,
            result: None,
            available: true,
            dependencies_ready: 0,
        }
    }

    pub fn status(&self) -> TaskStatus {
        if self.result.is_some() {
            return TaskStatus::Done;
        }

        if self.config.is_none() {
            return TaskStatus::Resolving;
        }

        if self.dependencies_ready < self.dependencies.len() {
            return TaskStatus::Blocked;
        }

        if self.result.is_none() {
            return TaskStatus::Building;
        }

        TaskStatus::Done
    }
}

pub trait ResolverPlugin: std::fmt::Debug {
    fn can_resolve(&self, target: &str) -> bool;
    fn resolve(&self, context: Context, target: &str) -> std::io::Result<Config>;
}

pub trait BuildPlugin: std::fmt::Debug {
    fn build(
        &self,
        context: Context,
        task: Task,
        dependencies: HashMap<String, BuildOutput>,
    ) -> BuildResult;
}

#[derive(Debug)]
pub struct FakeBuilder {}

impl BuildPlugin for FakeBuilder {
    fn build(
        &self,
        context: Context,
        task: Task,
        dependencies: HashMap<String, BuildOutput>,
    ) -> BuildResult {
        BuildResult::noop()
    }
}

#[derive(Debug)]
pub struct FakeResolver {
    configs: HashMap<String, std::io::Result<Config>>,
}

impl FakeResolver {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    pub fn with_configs(configs: Vec<(&str, std::io::Result<Config>)>) -> Self {
        Self {
            configs: configs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }
}

impl ResolverPlugin for FakeResolver {
    fn can_resolve(&self, target: &str) -> bool {
        true
    }

    fn resolve(&self, context: Context, target: &str) -> std::io::Result<Config> {
        match self.configs.get(target) {
            Some(Ok(c)) => Ok(c.clone()),
            Some(Err(e)) => Err(std::io::Error::new(e.kind(), "failed to read config")),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("failed to resolve target {target}"),
            )),
        }
    }
}

#[derive(Debug)]
pub struct FilesystemBuilder {}

impl BuildPlugin for FilesystemBuilder {
    fn build(
        &self,
        context: Context,
        task: Task,
        deps: HashMap<String, BuildOutput>,
    ) -> BuildResult {
        let loc = match task
            .config
            .expect("config must be resolved by now")
            .location
        {
            Some(l) => l,
            None => {
                return BuildResult::Failure(
                    "filesystem builder plugin requires a location set in the build config"
                        .to_string(),
                )
            }
        };
        BuildResult::Success(BuildOutput {
            outputs: vec![std::path::PathBuf::from(loc)],
        })
    }
}

pub fn target_shortname(target: &str) -> &str {
    target
        .split("//")
        .last()
        .and_then(|s| s.split("/").last())
        .and_then(|s| s.split(":").last())
        .unwrap_or("")
}

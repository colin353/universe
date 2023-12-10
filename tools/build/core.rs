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
    Success { outputs: Vec<std::path::PathBuf> },
    Failure(String),
}

impl BuildResult {
    pub fn noop() -> Self {
        BuildResult::Success {
            outputs: Vec::new(),
        }
    }

    pub fn merged<'a, I: Iterator<Item = &'a Self>>(results: I) -> Self {
        let mut outs = Vec::new();
        for result in results {
            match result {
                BuildResult::Success { outputs } => {
                    outs.extend(outputs.to_owned());
                }
                _ => return result.clone(),
            }
        }
        BuildResult::Success { outputs: outs }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub dependencies: Vec<String>,
    pub build_plugin: String,
    pub location: Option<String>,
}

impl Config {
    pub fn dependencies(&self) -> Vec<&str> {
        let mut out: Vec<_> = self.dependencies.iter().map(|s| s.as_str()).collect();
        out.push(self.build_plugin.as_str());
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
    fn resolve(&self, target: &str) -> std::io::Result<Config>;
}

pub trait BuildPlugin: std::fmt::Debug {
    fn build(&self, task: Task) -> BuildResult;
}

#[derive(Debug)]
pub struct FakeBuilder {}

impl BuildPlugin for FakeBuilder {
    fn build(&self, task: Task) -> BuildResult {
        BuildResult::noop()
    }
}

#[derive(Debug)]
pub struct FakeResolver {}

impl ResolverPlugin for FakeResolver {
    fn can_resolve(&self, target: &str) -> bool {
        true
    }

    fn resolve(&self, target: &str) -> std::io::Result<Config> {
        if target == "//:builder" {
            return Ok(Config {
                build_plugin: "@filesystem".to_string(),
                location: Some("/tmp/file.txt".to_string()),
                ..Default::default()
            });
        }

        Ok(Config {
            build_plugin: "//:builder".to_string(),
            ..Default::default()
        })
    }
}

#[derive(Debug)]
pub struct FilesystemBuilder {}

impl BuildPlugin for FilesystemBuilder {
    fn build(&self, task: Task) -> BuildResult {
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
        BuildResult::Success {
            outputs: vec![std::path::PathBuf::from(loc)],
        }
    }
}

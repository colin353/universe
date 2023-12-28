use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::*;
use crate::plugins::PluginKind;

#[derive(Debug)]
pub struct Executor {
    context: Context,
    tasks: Mutex<TaskGraph>,

    resolvers: Vec<Box<dyn ResolverPlugin>>,
    builders: Mutex<HashMap<String, Arc<dyn BuildPlugin>>>,
}

#[derive(Debug)]
pub struct TaskGraph {
    tasks: Vec<Task>,
    by_target: HashMap<String, usize>,
    rdeps: Vec<Vec<usize>>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            context: Context::new(std::path::PathBuf::from("/tmp/cache")),
            tasks: Mutex::new(TaskGraph::new()),

            resolvers: Vec::new(),
            builders: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_task<T: Into<String>>(&self, target: T, rdep: Option<usize>) -> usize {
        let target: String = target.into();
        let is_builder = self.builders.lock().unwrap().contains_key(&target);
        let mut graph = self.tasks.lock().unwrap();
        let exists = graph.by_target.contains_key(&target);
        let id = graph.add_task(target, rdep);
        if !exists && is_builder {
            graph.mark_build_success(id, BuildResult::noop());
        }
        id
    }

    pub fn next_task(&self) -> Option<Task> {
        let mut graph = self.tasks.lock().unwrap();
        for task in &mut graph.tasks {
            if task.available {
                match task.status() {
                    TaskStatus::Resolving | TaskStatus::Building => {
                        // Mark the task as unavailable
                        task.available = false;
                        return Some(task.clone());
                    }
                    TaskStatus::Blocked | TaskStatus::Done => continue,
                }
            }
        }
        None
    }

    pub fn run(&self, roots: &[usize]) -> BuildResult {
        while let Some(task) = self.next_task() {
            match task.status() {
                TaskStatus::Resolving => self.resolve(task),
                TaskStatus::Building => self.build(task),
                TaskStatus::Blocked | TaskStatus::Done => {
                    unreachable!("cannot acquire a blocked or done task!")
                }
            }
        }

        // All tasks must be built by now...
        let graph = self.tasks.lock().unwrap();
        for task in &graph.tasks {
            if task.status() != TaskStatus::Done {
                return BuildResult::Failure(format!(
                    "not all tasks finished, deadlock! still waiting on {task:?}",
                ));
            }

            match &task.result {
                Some(BuildResult::Success { .. }) => continue,
                Some(BuildResult::Failure(reason)) => {
                    return BuildResult::Failure(reason.to_string());
                }
                None => {
                    return BuildResult::Failure(String::from("not all tasks produced a result"))
                }
            }
        }

        BuildResult::merged(roots.iter().map(|r| {
            graph.tasks[*r]
                .result
                .as_ref()
                .expect("result must be available")
        }))
    }

    pub fn resolve(&self, task: Task) {
        for resolver in &self.resolvers {
            if !resolver.can_resolve(&task.target) {
                continue;
            }
            match resolver.resolve(self.context.with_target(&task.target), &task.target) {
                Ok(config) => {
                    // Add all dependent tasks first
                    let deps: Vec<usize> = config
                        .dependencies()
                        .into_iter()
                        .map(|t| self.add_task(t, Some(task.id)))
                        .collect();

                    let mut graph = self.tasks.lock().unwrap();

                    // It's possible that some of the dependencies are already ready, so pre-set
                    // the right ready count.
                    let dependencies_ready = deps
                        .iter()
                        .filter(|id| graph.tasks[**id].status() == TaskStatus::Done)
                        .count();

                    let mut t = &mut graph.tasks[task.id];
                    t.dependencies = deps;
                    t.dependencies_ready = dependencies_ready;

                    t.config = Some(config);
                    t.available = true;
                }
                Err(e) => {
                    self.mark_task_failure(
                        task.id,
                        BuildResult::Failure(format!("target resolution failed: {e:#?}")),
                    );
                }
            }
            return;
        }

        // No resolver available for the target!
        self.mark_task_failure(
            task.id,
            BuildResult::Failure(format!("no resolver available for target {}", task.target)),
        );
    }

    pub fn build(&self, task: Task) {
        let config = task
            .config
            .as_ref()
            .expect("must have config resolved before build can begin!");

        let plugin = {
            let mut builders = self.builders.lock().unwrap();
            if let Some(p) = builders.get(&config.build_plugin) {
                p.clone()
            } else {
                // Load the plugin from built dependencies
                let graph = self.tasks.lock().unwrap();
                let plugin_task = match graph.by_target.get(&config.build_plugin) {
                    Some(t) => &graph.tasks[*t],
                    None => {
                        panic!("we must have already loaded this plugin's target by now!");
                    }
                };
                let plugin_path = match plugin_task
                    .result
                    .as_ref()
                    .expect("this plugin must already have been built!")
                {
                    BuildResult::Success(BuildOutput { outputs, .. }) => outputs[0].clone(),
                    _ => panic!("the plugin build must have succeeded by now!"),
                };
                let plugin = load_plugin(&plugin_path);
                builders.insert(config.build_plugin.clone(), plugin.clone());
                plugin
            }
        };

        let mut deps = HashMap::new();
        {
            let graph = self.tasks.lock().unwrap();
            for dep in task
                .config
                .as_ref()
                .expect("task must have config defined by now!")
                .dependencies()
            {
                let dt = match graph.by_target.get(dep) {
                    Some(t) => &graph.tasks[*t],
                    None => {
                        panic!("all dependencies must exist by now!");
                    }
                };

                match dt.result.as_ref() {
                    Some(BuildResult::Success(out)) => {
                        deps.insert(dt.target.clone(), out.clone());
                    }
                    Some(BuildResult::Failure(_)) => {
                        panic!("all dependencies must be succesfully built by now!");
                    }
                    None => panic!("all dependencies must be finished building by now!"),
                }
            }
        }

        let result = plugin.build(self.context.with_task(&task), task.clone(), deps);
        match result {
            BuildResult::Success { .. } => {
                self.mark_build_success(task.id, result);
            }
            BuildResult::Failure(_) => {
                self.mark_task_failure(task.id, result);
            }
        }
    }

    pub fn mark_task_failure(&self, id: usize, result: BuildResult) {
        {
            // The task failed, so we can log the reason:
            let graph = self.tasks.lock().unwrap();
            let task = graph.tasks.get(id).unwrap();
            let stage = match task.failure_stage() {
                TaskStatus::Resolving => "resolve",
                TaskStatus::Building => "build",
                _ => "??",
            };
            println!("\nfailed to {} {}:\n", stage, task.target);
            if let Some(msgs) = self.context.logs.read().unwrap().get(&task.target) {
                for msg in msgs.lock().unwrap().iter() {
                    println!("{}", msg);
                }
            }

            if let BuildResult::Failure(ref msg) = result {
                println!("{msg}");
            }
        }

        self.tasks.lock().unwrap().mark_task_failure(id, id, result);
    }

    pub fn print_all_logs(&self) {
        for (target, logs) in self.context.logs.read().unwrap().iter() {
            println!("\nlogs from {}:\n", target);
            for msg in logs.lock().unwrap().iter() {
                println!("{}", msg);
            }
        }
    }

    pub fn mark_build_success(&self, id: usize, result: BuildResult) {
        self.tasks.lock().unwrap().mark_build_success(id, result);
    }
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            by_target: HashMap::new(),
            rdeps: Vec::new(),
        }
    }

    pub fn add_task<T: Into<String>>(&mut self, target: T, rdep: Option<usize>) -> usize {
        let target: String = target.into();

        if let Some(id) = self.by_target.get(&target) {
            if let Some(r) = rdep {
                self.rdeps[*id].push(r);
            }

            return *id;
        }

        match rdep {
            Some(r) => self.rdeps.push(vec![r]),
            None => self.rdeps.push(Vec::new()),
        }

        let id = self.tasks.len();
        self.tasks.push(Task::new(id, target.clone()));
        self.by_target.insert(target.into(), id);
        id
    }

    pub fn mark_task_failure(&mut self, id: usize, root_cause: usize, result: BuildResult) {
        self.tasks[id].result = Some(result);
        self.tasks[id].available = true;
        for rdep in self.rdeps[id].clone() {
            self.mark_task_failure(
                rdep,
                root_cause,
                BuildResult::Failure(format!(
                    "failed to build dependency: {}",
                    self.tasks[root_cause].target
                )),
            );
        }
    }

    pub fn mark_build_success(&mut self, id: usize, result: BuildResult) {
        self.tasks[id].result = Some(result);
        self.tasks[id].available = true;
        for rdep in &self.rdeps[id] {
            self.tasks[*rdep].dependencies_ready += 1;
        }
    }
}

fn load_plugin(path: &std::path::Path) -> Arc<dyn BuildPlugin> {
    if let Some(f) = path.file_name() {
        if f == "rust.cdylib" {
            return Arc::new(crate::plugins::RustPlugin {});
        }
    }

    Arc::new(FakeBuilder {})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo::CargoResolver;

    #[test]
    fn test_execution() {
        let mut e = Executor::new();
        e.builders
            .lock()
            .unwrap()
            .insert("@filesystem".to_string(), Arc::new(FilesystemBuilder {}));
        e.resolvers.push(Box::new(FakeResolver::with_configs(vec![
            (
                "//:builder",
                Ok(Config {
                    build_plugin: "@filesystem".to_string(),
                    location: Some("/tmp/file.txt".to_string()),
                    ..Default::default()
                }),
            ),
            (
                "//:my_target",
                Ok(Config {
                    build_plugin: "//:builder".to_string(),
                    ..Default::default()
                }),
            ),
        ])));
        let id = e.add_task("//:my_target", None);
        let result = e.run(&[id]);
        assert_eq!(result, BuildResult::noop());
    }

    //#[test]
    fn test_library_build() {
        let mut e = Executor::new();
        e.builders
            .lock()
            .unwrap()
            .insert("@filesystem".to_string(), Arc::new(FilesystemBuilder {}));

        e.resolvers.push(Box::new(FakeResolver::with_configs(vec![
            (
                "//:lhello",
                Ok(Config {
                    build_plugin: "@rust_plugin".to_string(),
                    sources: vec!["/tmp/lhello.rs".to_string()],
                    build_dependencies: vec!["@rust_compiler".to_string()],
                    kind: PluginKind::RustLibrary.to_string(),
                    ..Default::default()
                }),
            ),
            (
                "//:my_program",
                Ok(Config {
                    build_plugin: "@rust_plugin".to_string(),
                    sources: vec!["/tmp/hello_world.rs".to_string()],
                    dependencies: vec!["//:lhello".to_string()],
                    build_dependencies: vec!["@rust_compiler".to_string()],
                    kind: PluginKind::RustBinary.to_string(),
                    ..Default::default()
                }),
            ),
            (
                "@rust_plugin",
                Ok(Config {
                    build_plugin: "@filesystem".to_string(),
                    location: Some("/tmp/rust.cdylib".to_string()),
                    ..Default::default()
                }),
            ),
            (
                "@rust_compiler",
                Ok(Config {
                    build_plugin: "@filesystem".to_string(),
                    location: Some("/Users/colinwm/.cargo/bin/rustc".to_string()),
                    ..Default::default()
                }),
            ),
        ])));

        let id = e.add_task("//:my_program", None);
        let result = e.run(&[id]);
        assert_eq!(
            result,
            BuildResult::Success(BuildOutput {
                outputs: vec![std::path::PathBuf::from("/tmp/a.out")],
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_cargo_build() {
        let mut e = Executor::new();
        e.builders
            .lock()
            .unwrap()
            .insert("@filesystem".to_string(), Arc::new(FilesystemBuilder {}));

        e.context.lockfile = Arc::new(
            vec![
                ("cargo://rand".to_string(), "0.8.5".to_string()),
                ("cargo://rand_core".to_string(), "0.6.0".to_string()),
                ("cargo://libc".to_string(), "0.2.151".to_string()),
                ("cargo://getrandom".to_string(), "0.2.11".to_string()),
                ("cargo://cfg-if".to_string(), "1.0.0".to_string()),
                ("cargo://rand_chacha".to_string(), "0.3.1".to_string()),
                ("cargo://ppv-lite86".to_string(), "0.2.17".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        e.resolvers.push(Box::new(CargoResolver::new()));

        e.resolvers.push(Box::new(FakeResolver::with_configs(vec![
            (
                "@rust_compiler",
                Ok(Config {
                    build_plugin: "@filesystem".to_string(),
                    location: Some("/Users/colinwm/.cargo/bin/rustc".to_string()),
                    ..Default::default()
                }),
            ),
            (
                "@rust_plugin",
                Ok(Config {
                    build_plugin: "@filesystem".to_string(),
                    location: Some("/tmp/rust.cdylib".to_string()),
                    ..Default::default()
                }),
            ),
            (
                "//:dice_roll",
                Ok(Config {
                    build_plugin: "@rust_plugin".to_string(),
                    sources: vec!["/tmp/dice_roll.rs".to_string()],
                    dependencies: vec!["cargo://rand".to_string()],
                    build_dependencies: vec!["@rust_compiler".to_string()],
                    kind: PluginKind::RustBinary.to_string(),
                    ..Default::default()
                }),
            ),
        ])));

        let id = e.add_task("//:dice_roll", None);
        let result = e.run(&[id]);

        e.print_all_logs();

        assert_eq!(
            result,
            BuildResult::Success(BuildOutput {
                outputs: vec![std::path::PathBuf::from("/tmp/a.out")],
                ..Default::default()
            })
        );
    }
}

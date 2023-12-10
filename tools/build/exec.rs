use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::*;

#[derive(Debug)]
pub struct ExecutionContext {
    start_time: std::time::Instant,
}

#[derive(Debug)]
pub struct Executor {
    context: ExecutionContext,
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
            context: ExecutionContext {
                start_time: std::time::Instant::now(),
            },
            tasks: Mutex::new(TaskGraph::new()),

            resolvers: Vec::new(),
            builders: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_task<T: Into<String>>(&self, target: T, rdep: Option<usize>) -> usize {
        let target: String = target.into();
        let is_builder = self.builders.lock().unwrap().contains_key(&target);
        let mut graph = self.tasks.lock().unwrap();
        let id = graph.add_task(target, rdep);
        if is_builder {
            graph.mark_build_success(id, BuildResult::noop());
        }
        id
    }

    pub fn next_task(&self) -> Option<Task> {
        let mut graph = self.tasks.lock().unwrap();
        for task in &mut graph.tasks {
            println!("check on {:#?} (status = {:?})", task, task.status());
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
            match resolver.resolve(&task.target) {
                Ok(config) => {
                    // Add all dependent tasks first
                    let deps: Vec<usize> = config
                        .dependencies()
                        .into_iter()
                        .map(|t| self.add_task(t, Some(task.id)))
                        .collect();

                    let mut graph = self.tasks.lock().unwrap();
                    let mut t = &mut graph.tasks[task.id];
                    t.dependencies = deps;
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
                    BuildResult::Success { outputs } => outputs[0].clone(),
                    _ => panic!("the plugin build must have succeeded by now!"),
                };
                let plugin = load_plugin(&plugin_path);
                builders.insert(config.build_plugin.clone(), plugin.clone());
                plugin
            }
        };

        let result = plugin.build(task.clone());
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
        self.tasks.lock().unwrap().mark_task_failure(id, id, result);
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
            return *id;
        }

        let id = self.tasks.len();
        self.tasks.push(Task::new(id, target.clone()));
        self.by_target.insert(target.into(), id);
        match rdep {
            Some(r) => self.rdeps.push(vec![r]),
            None => self.rdeps.push(Vec::new()),
        }
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
    Arc::new(FakeBuilder {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution() {
        let mut e = Executor::new();
        e.builders
            .lock()
            .unwrap()
            .insert("@filesystem".to_string(), Arc::new(FilesystemBuilder {}));
        e.resolvers.push(Box::new(FakeResolver {}));
        let id = e.add_task("//:my_target", None);
        let result = e.run(&[id]);
        assert_eq!(result, BuildResult::noop());
    }
}

extern crate tasks_grpc_rust;
extern crate tokio;
#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use tasks_grpc_rust::{Status, TaskArgument, TaskStatus};
use tokio::prelude::{future, Future};

lazy_static! {
    pub static ref TASK_REGISTRY: HashMap<&'static str, Box<dyn Task>> = {
        let mut m: HashMap<&'static str, Box<dyn Task>> = HashMap::new();
        m.insert("noop", Box::new(Noop::new()));
        m.insert("spawner", Box::new(Spawner::new()));
        m
    };
}

pub type TaskResultFuture = Box<dyn Future<Item = TaskStatus, Error = ()> + Send>;

pub trait TaskManager: Send {
    fn set_status(&self, status: &TaskStatus);
    fn get_status(&self) -> TaskStatus;
    fn spawn(&self, task_name: &str, arguments: Vec<TaskArgument>) -> TaskResultFuture;
    fn run(self, mut status: TaskStatus) -> TaskResultFuture;
    fn failure(&self, mut status: TaskStatus, reason: &str) -> TaskResultFuture {
        status.set_status(Status::FAILURE);
        status.set_reason(reason.to_owned());
        Box::new(future::ok(status))
    }
    fn success(&self, mut status: TaskStatus) -> TaskResultFuture {
        status.set_status(Status::SUCCESS);
        Box::new(future::ok(status))
    }
}

pub trait Task: Sync + 'static {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture;
}

pub struct Noop {}
impl Noop {
    fn new() -> Self {
        Self {}
    }
}
impl Task for Noop {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        let mut status = manager.get_status();
        status.set_status(Status::SUCCESS);
        Box::new(future::ok(status))
    }
}

pub struct Spawner {}
impl Spawner {
    fn new() -> Self {
        Self {}
    }
}
impl Task for Spawner {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        let mut status = manager.get_status();

        if args.len() != 1 {
            return manager.failure(status, "not enough arguments");
        }

        return Box::new(
            manager
                .spawn(args[0].get_value_string(), Vec::new())
                .and_then(move |s| {
                    if s.get_status() != Status::SUCCESS {
                        return manager.failure(status, "subtask did not succeed");
                    }

                    manager.success(status)
                }),
        );
    }
}

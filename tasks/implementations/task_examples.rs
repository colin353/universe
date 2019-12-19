extern crate task_lib;
extern crate tasks_grpc_rust;
extern crate tokio;

use task_lib::{Task, TaskManager, TaskResultFuture};
use tasks_grpc_rust::{Status, TaskArgument};
use tokio::prelude::{future, Future};

pub struct Noop {}
impl Noop {
    pub fn new() -> Self {
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
    pub fn new() -> Self {
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

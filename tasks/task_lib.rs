extern crate tasks_grpc_rust;
extern crate tokio;

use std::collections::HashMap;
use tasks_grpc_rust::{Status, TaskArgument, TaskStatus};
use tokio::prelude::{future, Future};

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

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
        m
    };
}

pub type TaskResultFuture = Box<dyn Future<Item = TaskStatus, Error = ()> + Send>;

pub trait TaskManager {
    fn set_status(&self, status: TaskStatus);
    fn get_status(&self) -> TaskStatus;
    fn run(self, mut status: TaskStatus) -> TaskResultFuture;
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

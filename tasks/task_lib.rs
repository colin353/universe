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

pub trait TaskManager {
    fn set_status(&self, status: TaskStatus);
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

pub struct ArgumentsBuilder {
    args: Vec<TaskArgument>,
}

impl ArgumentsBuilder {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub fn add_string(&mut self, name: &str, value: String) {
        let mut a = TaskArgument::new();
        a.set_name(name.to_owned());
        a.set_value_string(value);
        self.args.push(a)
    }

    pub fn add_int(&mut self, name: &str, value: i64) {
        let mut a = TaskArgument::new();
        a.set_name(name.to_owned());
        a.set_value_int(value);
        self.args.push(a)
    }

    pub fn add_float(&mut self, name: &str, value: f32) {
        let mut a = TaskArgument::new();
        a.set_name(name.to_owned());
        a.set_value_float(value);
        self.args.push(a)
    }

    pub fn add_bool(&mut self, name: &str, value: bool) {
        let mut a = TaskArgument::new();
        a.set_name(name.to_owned());
        a.set_value_bool(value);
        self.args.push(a)
    }

    pub fn build(self) -> Vec<TaskArgument> {
        self.args
    }
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

        return manager.spawn(args[0].get_value_string(), Vec::new());
    }
}

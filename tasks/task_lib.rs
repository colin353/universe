extern crate tasks_grpc_rust;
extern crate tokio;

use std::collections::HashMap;
use tasks_grpc_rust::{Status, TaskArgument, TaskArtifact, TaskStatus};
use tokio::prelude::{future, Future};

pub type TaskResultFuture = Box<dyn Future<Item = TaskStatus, Error = ()> + Send>;

pub trait TaskManager: Send + Sync {
    fn set_status(&self, status: &TaskStatus);
    fn get_status(&self) -> TaskStatus;
    fn spawn(&self, task_name: &str, arguments: Vec<TaskArgument>) -> TaskResultFuture;
    fn run(self, status: TaskStatus) -> TaskResultFuture;
    fn failure(&self, mut status: TaskStatus, reason: &str) -> TaskResultFuture {
        status.set_status(Status::FAILURE);
        status.set_reason(reason.to_owned());
        Box::new(future::ok(status))
    }
    fn success(&self, mut status: TaskStatus) -> TaskResultFuture {
        status.set_status(Status::SUCCESS);
        Box::new(future::ok(status))
    }
    fn get_configuration(&self) -> &TaskServerConfiguration;
}

pub trait Task: Sync + 'static {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture;
}

#[derive(Clone)]
pub struct TaskServerConfiguration {
    pub weld_client_hostname: String,
    pub weld_client_port: u16,
    pub weld_server_hostname: String,
    pub weld_server_port: u16,
    pub base_url: String,
}

impl TaskServerConfiguration {
    pub fn new() -> Self {
        Self {
            weld_client_hostname: String::new(),
            weld_client_port: 0,
            base_url: String::new(),
            weld_server_hostname: String::new(),
            weld_server_port: 0,
        }
    }
}

pub struct ArtifactBuilder;
impl ArtifactBuilder {
    pub fn from_string(name: &str, value: String) -> TaskArtifact {
        let mut a = TaskArtifact::new();
        a.set_name(name.to_owned());
        a.set_value_string(value);
        a
    }

    pub fn from_int(name: &str, value: i64) -> TaskArtifact {
        let mut a = TaskArtifact::new();
        a.set_name(name.to_owned());
        a.set_value_int(value);
        a
    }

    pub fn from_bool(name: &str, value: bool) -> TaskArtifact {
        let mut a = TaskArtifact::new();
        a.set_name(name.to_owned());
        a.set_value_bool(value);
        a
    }
}

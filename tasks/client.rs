extern crate grpc;
extern crate largetable_client;
extern crate protobuf;
extern crate tasks_grpc_rust;

use largetable_client::LargeTableClient;
use std::sync::Arc;
use tasks_grpc_rust::TaskService;
use tasks_grpc_rust::{CreateTaskRequest, GetStatusRequest, Status, TaskArgument, TaskStatus};

const TASK_IDS: &'static str = "task_ids";
const TASK_STATUS: &'static str = "task_status";

#[derive(Clone)]
pub struct TaskClient<C: LargeTableClient + Clone + 'static> {
    database: C,
}

impl<C: LargeTableClient + Clone + 'static> TaskClient<C> {
    pub fn new(db: C) -> Self {
        Self { database: db }
    }

    pub fn write(&self, status: &TaskStatus) {
        self.database
            .write_proto(TASK_STATUS, status.get_task_id(), 0, status);
    }

    pub fn read(&self, task_id: &str) -> Option<TaskStatus> {
        self.database.read_proto(TASK_STATUS, task_id, 0)
    }

    pub fn reserve_task_id(&self) -> String {
        self.database.reserve_id(TASK_IDS, "").to_string()
    }

    pub fn reserve_subtask_id(&self, task_id: &str) -> String {
        let id = self.database.reserve_id(TASK_IDS, task_id);
        format!("s{}/{}", task_id, id)
    }

    pub fn list_subtasks<'a>(&'a self, task_id: &str) -> impl Iterator<Item = TaskStatus> + 'a {
        largetable_client::LargeTableScopedIterator::<'a, TaskStatus, C>::new(
            &self.database,
            String::from(TASK_STATUS),
            format!("s{}/", task_id),
            String::new(),
            String::new(),
            0,
        )
        .map(|(_key, val)| val)
    }

    pub fn list_tasks<'a>(&'a self) -> impl Iterator<Item = TaskStatus> + 'a {
        largetable_client::LargeTableScopedIterator::<'a, TaskStatus, C>::new(
            &self.database,
            String::from(TASK_STATUS),
            String::new(),
            String::new(),
            String::new(),
            0,
        )
        .map(|(_key, val)| val)
    }
}

#[derive(Clone)]
pub struct TaskRemoteClient {
    client: Arc<tasks_grpc_rust::TaskServiceClient>,
}

impl TaskRemoteClient {
    pub fn new(hostname: String, port: u16) -> Self {
        Self {
            client: Arc::new(
                tasks_grpc_rust::TaskServiceClient::new_plain(
                    &hostname,
                    port,
                    std::default::Default::default(),
                )
                .unwrap(),
            ),
        }
    }

    fn opts(&self) -> grpc::RequestOptions {
        grpc::RequestOptions::new()
    }

    pub fn create_task(&self, task_name: String, args: Vec<TaskArgument>) {
        let mut req = CreateTaskRequest::new();
        req.set_task_name(task_name);
        req.set_arguments(protobuf::RepeatedField::from_vec(args));
        self.client
            .create_task(self.opts(), req)
            .wait()
            .expect("rpc");
    }

    pub fn get_status(&self, task_id: String) -> Option<TaskStatus> {
        let mut req = GetStatusRequest::new();
        req.set_task_id(task_id);
        let response = self
            .client
            .get_status(self.opts(), req)
            .wait()
            .expect("rpc")
            .1;
        if response.get_status() == Status::DOES_NOT_EXIST {
            return None;
        }

        Some(response)
    }
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

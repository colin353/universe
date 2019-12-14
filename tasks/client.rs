extern crate largetable_client;
extern crate tasks_grpc_rust;

use largetable_client::LargeTableClient;
use tasks_grpc_rust::{Status, TaskArgument, TaskStatus};

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

    pub fn list_tasks<'a>(&'a self, task_id: &str) -> impl Iterator<Item = TaskStatus> + 'a {
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

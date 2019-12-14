extern crate grpc;
extern crate protobuf;
extern crate tokio;

extern crate futures;
extern crate largetable_client;
extern crate task_lib;
extern crate tasks_grpc_rust;

use futures::sync::mpsc::{unbounded, UnboundedSender};
use futures::Stream;
use largetable_client::LargeTableClient;
use task_lib::TaskManager;
use tasks_grpc_rust::{Status, TaskArgument, TaskStatus};
use tokio::prelude::{future, Future};

pub type TaskFuture = Box<dyn Future<Item = (), Error = ()> + Send>;

const TASK_IDS: &'static str = "task_ids";
const TASK_STATUS: &'static str = "task_status";

#[derive(Clone)]
pub struct TaskServiceHandler<C: LargeTableClient> {
    database: C,
    scheduler: UnboundedSender<String>,
}

impl<C: LargeTableClient + Clone + Send + 'static> TaskServiceHandler<C> {
    pub fn new(database: C) -> Self {
        let (sender, mut receiver) = unbounded();

        let handler = Self {
            database: database,
            scheduler: sender,
        };

        let h = handler.clone();
        std::thread::spawn(move || {
            let task_runner = receiver
                .map(move |m| tokio::spawn(h.begin_task(m)))
                .into_future()
                .map(|_| ())
                .map_err(|_| ());
            tokio::run(task_runner);
        });

        handler
    }

    pub fn create_task(
        &self,
        mut req: tasks_grpc_rust::CreateTaskRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        let mut initial_status = TaskStatus::new();
        initial_status.set_name(req.take_task_name());
        initial_status.set_arguments(req.take_arguments());
        let id = self.database.reserve_id(TASK_IDS, "").to_string();
        initial_status.set_task_id(id.clone());

        self.database
            .write_proto(TASK_STATUS, &id, 0, &initial_status);

        self.scheduler.unbounded_send(id);
        initial_status
    }

    pub fn get_status(
        &self,
        req: tasks_grpc_rust::GetStatusRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        let mut status: TaskStatus =
            match self.database.read_proto(TASK_STATUS, req.get_task_id(), 0) {
                Some(s) => s,
                None => {
                    let mut s = TaskStatus::new();
                    s.set_status(Status::DOES_NOT_EXIST);
                    return s;
                }
            };

        let iterator = largetable_client::LargeTableScopedIterator::new(
            &self.database,
            String::from(TASK_STATUS),
            format!("s{}/", req.get_task_id()),
            String::new(),
            String::new(),
            0,
        );
        for (_, subtask_status) in iterator {
            status.mut_subtasks().push(subtask_status);
        }

        status
    }

    pub fn begin_task(&self, task_id: String) -> TaskFuture {
        let status: TaskStatus = match self.database.read_proto(TASK_STATUS, &task_id, 0) {
            Some(s) => s,
            None => {
                let mut s = TaskStatus::new();
                s.set_task_id(task_id.clone());
                s.set_status(Status::DOES_NOT_EXIST);
                self.database.write_proto(TASK_STATUS, &task_id, 0, &s);
                return Box::new(future::ok(()));
            }
        };

        let m = Manager::new(task_id.clone(), self.database.clone());
        let db = self.database.clone();
        Box::new(m.run(status).map(move |status| {
            db.write_proto(TASK_STATUS, &task_id, 0, &status);
        }))
    }
}

impl<C: LargeTableClient + Clone + Send + 'static> tasks_grpc_rust::TaskService
    for TaskServiceHandler<C>
{
    fn create_task(
        &self,
        m: grpc::RequestOptions,
        req: tasks_grpc_rust::CreateTaskRequest,
    ) -> grpc::SingleResponse<tasks_grpc_rust::TaskStatus> {
        grpc::SingleResponse::completed(self.create_task(req))
    }

    fn get_status(
        &self,
        m: grpc::RequestOptions,
        req: tasks_grpc_rust::GetStatusRequest,
    ) -> grpc::SingleResponse<tasks_grpc_rust::TaskStatus> {
        grpc::SingleResponse::completed(self.get_status(req))
    }
}

struct Manager<C: LargeTableClient + Clone + 'static> {
    task_id: String,
    database: C,
}
impl<C: LargeTableClient + Clone + 'static> Manager<C> {
    pub fn new(task_id: String, database: C) -> Self {
        Self {
            task_id: task_id,
            database: database,
        }
    }
}

impl<C: LargeTableClient + Clone + 'static> task_lib::TaskManager for Manager<C> {
    fn get_status(&self) -> TaskStatus {
        self.database
            .read_proto(TASK_STATUS, &self.task_id, 0)
            .unwrap()
    }
    fn set_status(&self, status: TaskStatus) {
        self.database
            .write_proto(TASK_STATUS, &self.task_id, 0, &status);
    }
    fn spawn(&self, task_name: &str, arguments: Vec<TaskArgument>) -> task_lib::TaskResultFuture {
        let subtask_id = self
            .database
            .reserve_id(TASK_IDS, &self.task_id)
            .to_string();
        let task_id = format!("s{}/{}", self.task_id, subtask_id);
        let mut status = TaskStatus::new();
        status.set_name(task_name.to_owned());
        status.set_arguments(protobuf::RepeatedField::from_vec(arguments));
        status.set_task_id(task_id.clone());
        self.database.write_proto(TASK_STATUS, &task_id, 0, &status);

        let m = Manager::new(task_id.clone(), self.database.clone());
        m.run(status)
    }
    fn run(self, mut status: TaskStatus) -> task_lib::TaskResultFuture {
        let task = match task_lib::TASK_REGISTRY.get(status.get_name()) {
            Some(t) => t,
            None => {
                println!("Task not found");
                status.set_status(Status::FAILURE);
                let reason = format!("No registered task called `{}`", status.get_name());
                status.set_reason(reason);
                return Box::new(future::ok(status));
            }
        };

        task.run(status.get_arguments(), Box::new(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

    fn setup_task_runner() -> TaskServiceHandler<largetable_test::LargeTableMockClient> {
        let database = largetable_test::LargeTableMockClient::new();
        TaskServiceHandler::new(database)
    }

    #[test]
    fn test_task_running() {
        let handler = setup_task_runner();
        let mut req = tasks_grpc_rust::CreateTaskRequest::new();
        req.set_task_name(String::from("noop"));
        let status = handler.create_task(req);
        assert_eq!(status.get_task_id(), "1");
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut req = tasks_grpc_rust::GetStatusRequest::new();
        req.set_task_id(status.get_task_id().to_owned());
        let status = handler.get_status(req);
        assert_eq!(status.get_status(), Status::SUCCESS);
        assert_eq!(status.get_task_id(), "1");
    }

    #[test]
    fn test_task_spawning_fails_with_no_arguments() {
        let handler = setup_task_runner();
        let mut req = tasks_grpc_rust::CreateTaskRequest::new();
        req.set_task_name(String::from("spawner"));

        let status = handler.create_task(req);
        assert_eq!(status.get_task_id(), "1");

        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut req = tasks_grpc_rust::GetStatusRequest::new();
        req.set_task_id(status.get_task_id().to_owned());
        let status = handler.get_status(req);

        assert_eq!(status.get_status(), Status::FAILURE);
        assert_eq!(status.get_reason(), "not enough arguments");
        assert_eq!(status.get_task_id(), "1");
    }

    #[test]
    fn test_task_succeeds_spawning() {
        let handler = setup_task_runner();
        let mut req = tasks_grpc_rust::CreateTaskRequest::new();
        req.set_task_name(String::from("spawner"));
        let mut args = task_lib::ArgumentsBuilder::new();
        args.add_string("subtask", String::from("noop"));
        *req.mut_arguments() = protobuf::RepeatedField::from_vec(args.build());

        let status = handler.create_task(req);
        assert_eq!(status.get_task_id(), "1");

        std::thread::sleep(std::time::Duration::from_millis(50));

        let mut req = tasks_grpc_rust::GetStatusRequest::new();
        req.set_task_id(status.get_task_id().to_owned());
        let status = handler.get_status(req);

        assert_eq!(status.get_status(), Status::SUCCESS);
        assert_eq!(status.get_subtasks().len(), 1);
        assert_eq!(status.get_subtasks()[0].get_task_id(), "s1/1");
    }
}

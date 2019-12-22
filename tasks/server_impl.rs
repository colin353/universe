extern crate grpc;
extern crate protobuf;
extern crate tokio;

extern crate futures;
extern crate largetable_client;
extern crate registered_tasks;
extern crate task_client;
extern crate task_lib;
extern crate tasks_grpc_rust;

use futures::sync::mpsc::{unbounded, UnboundedSender};
use futures::Stream;
use largetable_client::LargeTableClient;
use task_client::{get_timestamp_usec, TaskClient};
use task_lib::{TaskManager, TaskServerConfiguration};
use tasks_grpc_rust::{Status, TaskArgument, TaskStatus};
use tokio::prelude::{future, Future};

pub type TaskFuture = Box<dyn Future<Item = (), Error = ()> + Send>;

#[derive(Clone)]
pub struct TaskServiceHandler<C: LargeTableClient + Clone + 'static> {
    client: TaskClient<C>,
    scheduler: UnboundedSender<String>,
    config: TaskServerConfiguration,
}

impl<C: LargeTableClient + Clone + Send + 'static> TaskServiceHandler<C> {
    pub fn new(config: TaskServerConfiguration, database: C) -> Self {
        let (sender, mut receiver) = unbounded();

        let handler = Self {
            client: TaskClient::new(database),
            scheduler: sender,
            config: config,
        };

        let h = handler.clone();
        std::thread::spawn(move || {
            let task_runner = receiver
                .for_each(move |m| tokio::spawn(h.begin_task(m)))
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
        let id = self.client.reserve_task_id();
        initial_status.set_task_id(id.clone());

        self.client.write(&initial_status);
        self.scheduler.unbounded_send(id);
        initial_status
    }

    pub fn get_status(
        &self,
        req: tasks_grpc_rust::GetStatusRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        match self.client.read(req.get_task_id()) {
            Some(s) => s,
            None => {
                let mut s = TaskStatus::new();
                s.set_status(Status::DOES_NOT_EXIST);
                return s;
            }
        }
    }

    pub fn begin_task(&self, task_id: String) -> TaskFuture {
        let status: TaskStatus = match self.client.read(&task_id) {
            Some(s) => s,
            None => {
                let mut s = TaskStatus::new();
                s.set_task_id(task_id.clone());
                s.set_status(Status::DOES_NOT_EXIST);
                self.client.write(&s);
                return Box::new(future::ok(()));
            }
        };

        let m = Manager::new(task_id.clone(), self.client.clone(), self.config.clone());
        Box::new(m.run(status).map(std::mem::drop))
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
    client: TaskClient<C>,
    config: TaskServerConfiguration,
}
impl<C: LargeTableClient + Clone + 'static> Manager<C> {
    pub fn new(task_id: String, client: TaskClient<C>, config: TaskServerConfiguration) -> Self {
        Self {
            task_id: task_id,
            client: client,
            config: config,
        }
    }
}

impl<C: LargeTableClient + Clone + Send + 'static> task_lib::TaskManager for Manager<C> {
    fn get_status(&self) -> TaskStatus {
        self.client.read(&self.task_id).unwrap()
    }

    fn set_status(&self, status: &TaskStatus) {
        self.client.write(&status)
    }

    fn spawn(&self, task_name: &str, arguments: Vec<TaskArgument>) -> task_lib::TaskResultFuture {
        let subtask_id = self.client.reserve_subtask_id(&self.task_id).to_string();
        let mut status = TaskStatus::new();
        status.set_name(task_name.to_owned());
        status.set_arguments(protobuf::RepeatedField::from_vec(arguments));
        status.set_task_id(subtask_id.clone());
        self.client.write(&status);

        let m = Manager::new(subtask_id.clone(), self.client.clone(), self.config.clone());
        let passed_client = self.client.clone();
        Box::new(m.run(status).and_then(move |s| {
            passed_client.write(&s);
            future::ok(s)
        }))
    }

    fn run(self, mut status: TaskStatus) -> task_lib::TaskResultFuture {
        let task = match registered_tasks::TASK_REGISTRY.get(status.get_name()) {
            Some(t) => t,
            None => {
                eprintln!("Task not found");
                status.set_status(Status::FAILURE);
                let reason = format!("No registered task called `{}`", status.get_name());
                status.set_reason(reason);
                return Box::new(future::ok(status));
            }
        };

        status.set_status(Status::STARTED);
        status.set_start_time(get_timestamp_usec());
        self.set_status(&status);

        let passed_client = self.client.clone();
        Box::new(
            task.run(status.get_arguments(), Box::new(self))
                .and_then(move |mut status| {
                    status.set_end_time(get_timestamp_usec());
                    passed_client.write(&status);
                    future::ok(status)
                }),
        )
    }

    fn get_configuration(&self) -> &TaskServerConfiguration {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

    fn setup_task_runner() -> TaskServiceHandler<largetable_test::LargeTableMockClient> {
        let database = largetable_test::LargeTableMockClient::new();
        let config = TaskServerConfiguration::new();
        TaskServiceHandler::new(config, database)
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

        // Schedule a second one
        let mut req = tasks_grpc_rust::CreateTaskRequest::new();
        req.set_task_name(String::from("noop"));
        let status = handler.create_task(req);
        assert_eq!(status.get_task_id(), "2");
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut req = tasks_grpc_rust::GetStatusRequest::new();
        req.set_task_id(status.get_task_id().to_owned());
        let status = handler.get_status(req);
        assert_eq!(status.get_status(), Status::SUCCESS);
        assert_eq!(status.get_task_id(), "2");
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
        let mut args = task_client::ArgumentsBuilder::new();
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

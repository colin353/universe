extern crate grpc;
extern crate tokio;

extern crate futures;
extern crate largetable_client;
extern crate tasks_grpc_rust;

use futures::sync::mpsc::{unbounded, UnboundedSender};
use futures::Stream;
use largetable_client::LargeTableClient;
use std::sync::{Arc, Mutex};
use tokio::prelude::{future, Future};

pub type TaskFuture = Box<dyn Future<Item = (), Error = ()> + Send>;

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
        req: tasks_grpc_rust::CreateTaskRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        self.scheduler.unbounded_send(String::from("msg"));
        tasks_grpc_rust::TaskStatus::new()
    }

    pub fn get_status(
        &self,
        req: tasks_grpc_rust::GetStatusRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        tasks_grpc_rust::TaskStatus::new()
    }

    pub fn begin_task(&self, task_id: String) -> TaskFuture {
        println!("begin task: {}", task_id);
        Box::new(future::ok(()))
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
        let req = tasks_grpc_rust::CreateTaskRequest::new();
        handler.create_task(req);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

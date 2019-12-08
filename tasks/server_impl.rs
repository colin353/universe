extern crate grpc;
extern crate largetable_client;
extern crate tasks_grpc_rust;

use largetable_client::LargeTableClient;

#[derive(Clone)]
pub struct TaskServiceHandler<C: LargeTableClient> {
    database: C,
}

impl<C: LargeTableClient + Clone> TaskServiceHandler<C> {
    pub fn new(db: C) -> Self {
        Self { database: db }
    }

    pub fn create_task(
        &self,
        req: tasks_grpc_rust::CreateTaskRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        tasks_grpc_rust::TaskStatus::new()
    }

    pub fn get_status(
        &self,
        req: tasks_grpc_rust::GetStatusRequest,
    ) -> tasks_grpc_rust::TaskStatus {
        tasks_grpc_rust::TaskStatus::new()
    }
}

impl<C: LargeTableClient + Clone> tasks_grpc_rust::TaskService for TaskServiceHandler<C> {
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

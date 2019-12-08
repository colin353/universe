extern crate grpc;
extern crate tasks_grpc_rust;

#[derive(Clone)]
pub struct TaskServiceHandler;

impl TaskServiceHandler {
    pub fn new() -> Self {
        Self {}
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

impl tasks_grpc_rust::TaskService for TaskServiceHandler {
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

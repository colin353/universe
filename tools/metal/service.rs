use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use metal_grpc_rust::{Configuration, DiffResponse, DiffType, Task, TaskRuntimeInfo, TaskState};
use state::{MetalStateError, MetalStateManager};

const RESOLUTION_TTL: u64 = 5;

pub struct MetalServiceHandler(pub Arc<MetalServiceHandlerInner>);

pub struct MetalServiceHandlerInner {
    tasks: RwLock<HashMap<String, Task>>,
    state: Arc<dyn MetalStateManager>,
    monitor: Arc<dyn core::Monitor>,
}

impl MetalServiceHandler {
    pub fn new(
        state: Arc<dyn MetalStateManager>,
        monitor: Arc<dyn core::Monitor>,
    ) -> Result<Self, MetalStateError> {
        // Initialize the task state by reading from any existing state
        let tasks = state
            .all_tasks()?
            .into_iter()
            .map(|t| (t.get_name().to_string(), t))
            .collect();

        Ok(MetalServiceHandler(Arc::new(MetalServiceHandlerInner {
            tasks: RwLock::new(tasks),
            monitor,
            state,
        })))
    }
}

impl core::Coordinator for MetalServiceHandlerInner {
    fn report_tasks(&self, tasks: Vec<(String, TaskRuntimeInfo)>) -> Vec<String> {
        let mut _tasks = self.tasks.write().unwrap();
        let mut to_clean_up = Vec::new();
        for (task_name, runtime_state) in tasks {
            let state = runtime_state.get_state();

            // Update current task status in state
            if let Some(t) = _tasks.get_mut(&task_name) {
                t.set_runtime_info(runtime_state);
                self.state.set_task(t);
            }

            match state {
                TaskState::SUCCESS | TaskState::STOPPED | TaskState::FAILED => {
                    // Task is done, remove it
                    _tasks.remove(&task_name);
                    to_clean_up.push(task_name.clone());
                }
                _ => (),
            };
        }

        to_clean_up
    }
}

fn compute_diff(
    current: &HashMap<String, Task>,
    desired: &Configuration,
    down: bool,
) -> DiffResponse {
    let mut response = DiffResponse::new();
    for task in desired.get_tasks() {
        if let Some(current_task) = current.get(task.get_name()) {
            if down {
                response.mut_removed().mut_tasks().push(task.clone());
            } else {
                let difference = diff::diff_task(&current_task, task);
                if difference.get_kind() == DiffType::MODIFIED {
                    response.mut_added().mut_tasks().push(task.clone());
                }
            }
        } else {
            if !down {
                response.mut_added().mut_tasks().push(task.clone());
            }
        }
    }
    response
}

impl MetalServiceHandlerInner {
    fn update(&self, req: metal_grpc_rust::UpdateRequest) -> metal_grpc_rust::UpdateResponse {
        let difference: DiffResponse;
        {
            let mut locked = self.tasks.write().unwrap();
            difference = compute_diff(&locked, req.get_config(), req.get_down());
            for task in difference.get_added().get_tasks() {
                if let Some(t) = locked.get_mut(task.get_name()) {
                    t.set_binary(task.get_binary().to_owned());
                    t.set_environment(task.get_environment().to_owned().into_iter().collect());
                    t.set_arguments(task.get_arguments().to_owned().into_iter().collect());
                    self.state.set_task(t).unwrap();
                } else {
                    locked.insert(task.get_name().to_owned(), task.to_owned());
                    self.state.set_task(task).unwrap();
                }
            }

            for task in difference.get_removed().get_tasks() {
                if let Some(t) = locked.get_mut(task.get_name()) {
                    t.set_environment(task.get_environment().to_owned().into_iter().collect());
                    self.state.set_task(t).unwrap();
                }
            }
        }

        let mut out = metal_grpc_rust::UpdateResponse::new();

        // Try to actually enact the difference using the monitor
        match self.monitor.execute(&difference) {
            Ok(_) => out.set_success(true),
            Err(e) => out.set_error_message(format!("failed to enact diff: {:?}", e)),
        }

        out.set_diff_applied(difference);
        out
    }

    fn diff(&self, req: metal_grpc_rust::UpdateRequest) -> metal_grpc_rust::DiffResponse {
        let locked = self.tasks.read().unwrap();
        compute_diff(&locked, req.get_config(), req.get_down())
    }

    fn resolve(&self, req: metal_grpc_rust::ResolveRequest) -> metal_grpc_rust::ResolveResponse {
        let mut response = metal_grpc_rust::ResolveResponse::new();

        // Try to resolve as a task + binding name
        if let Some(pos) = req.get_service_name().rfind('.') {
            let task_name = &req.get_service_name()[..pos];
            let binding_name = &req.get_service_name()[pos + 1..];
            if !task_name.is_empty() {
                let locked = self.tasks.read().unwrap();
                if let Some(t) = locked.get(task_name) {
                    for binding in t.get_runtime_info().get_services() {
                        if binding.get_service_name() == binding_name {
                            let mut endpoint = metal_grpc_rust::Endpoint::new();
                            endpoint
                                .set_ip_address(t.get_runtime_info().get_ip_address().to_owned());
                            endpoint.set_port(binding.get_port());

                            response.mut_endpoints().push(endpoint);
                        }
                    }
                }
            }
        }

        response
    }

    fn status(&self, req: metal_grpc_rust::StatusRequest) -> metal_grpc_rust::StatusResponse {
        let mut response = metal_grpc_rust::StatusResponse::new();
        for task in self.state.all_tasks().unwrap() {
            if task.get_name().starts_with(req.get_selector()) {
                response.mut_tasks().push(task);
            }
        }
        response
    }

    fn get_logs(&self, req: metal_grpc_rust::GetLogsRequest) -> metal_grpc_rust::GetLogsResponse {
        let mut response = metal_grpc_rust::GetLogsResponse::new();
        for log in self.monitor.get_logs(req.get_resource_name()).unwrap() {
            response.mut_logs().push(log);
        }
        response
    }
}

impl metal_grpc_rust::MetalService for MetalServiceHandler {
    fn update(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<metal_grpc_rust::UpdateRequest>,
        resp: grpc::ServerResponseUnarySink<metal_grpc_rust::UpdateResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.0.update(req.message))
    }

    fn diff(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<metal_grpc_rust::UpdateRequest>,
        resp: grpc::ServerResponseUnarySink<metal_grpc_rust::DiffResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.0.diff(req.message))
    }

    fn resolve(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<metal_grpc_rust::ResolveRequest>,
        resp: grpc::ServerResponseUnarySink<metal_grpc_rust::ResolveResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.0.resolve(req.message))
    }

    fn status(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<metal_grpc_rust::StatusRequest>,
        resp: grpc::ServerResponseUnarySink<metal_grpc_rust::StatusResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.0.status(req.message))
    }

    fn get_logs(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<metal_grpc_rust::GetLogsRequest>,
        resp: grpc::ServerResponseUnarySink<metal_grpc_rust::GetLogsResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.0.get_logs(req.message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use state::FakeState;

    #[test]
    fn test_simple_setup() {
        let state = Arc::new(FakeState::new());
        let monitor = Arc::new(core::FakeMonitor::new());
        let service = MetalServiceHandler::new(state, monitor).unwrap();

        let mut update = metal_grpc_rust::UpdateRequest::new();
        let mut t = Task::new();
        t.set_name(String::from("task_one"));
        update.mut_config().mut_tasks().push(t);
        let resp = service.0.update(update);

        assert_eq!(resp.get_diff_applied().get_added().get_tasks().len(), 1);

        // Now try to apply the same update again, and check diff
        let mut update = metal_grpc_rust::UpdateRequest::new();
        let mut t = Task::new();
        t.set_name(String::from("task_one"));
        update.mut_config().mut_tasks().push(t);
        let resp = service.0.update(update);

        assert_eq!(resp.get_diff_applied().get_added().get_tasks().len(), 0);

        // Now bring it down, should get removed
        let mut update = metal_grpc_rust::UpdateRequest::new();
        update.set_down(true);
        let mut t = Task::new();
        t.set_name(String::from("task_one"));
        update.mut_config().mut_tasks().push(t);
        let resp = service.0.update(update);

        assert_eq!(resp.get_diff_applied().get_removed().get_tasks().len(), 1);
        assert_eq!(resp.get_diff_applied().get_added().get_tasks().len(), 0);
    }
}

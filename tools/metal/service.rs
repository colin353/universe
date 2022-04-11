use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use diff::diff_task;
use metal_grpc_rust::{Configuration, DiffResponse, DiffType, Task, TaskState};
use state::{MetalStateError, MetalStateManager};

pub struct MetalServiceHandler {
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

        Ok(Self {
            tasks: RwLock::new(tasks),
            monitor,
            state,
        })
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
                if current_task.get_state() != TaskState::STOPPING {
                    response.mut_removed().mut_tasks().push(task.clone());
                }
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

impl MetalServiceHandler {
    fn update(&self, req: metal_grpc_rust::UpdateRequest) -> metal_grpc_rust::UpdateResponse {
        let mut locked = self.tasks.write().unwrap();
        let difference = compute_diff(&locked, req.get_config(), req.get_down());
        for task in difference.get_added().get_tasks() {
            if let Some(t) = locked.get_mut(task.get_name()) {
                t.set_binary(task.get_binary().to_owned());
                t.set_environment(task.get_environment().to_owned().into_iter().collect());
                t.set_arguments(task.get_arguments().to_owned().into_iter().collect());
                t.set_state(TaskState::RESTARTING);
                self.state.set_task(t);
            } else {
                locked.insert(task.get_name().to_owned(), task.to_owned());
                self.state.set_task(task);
            }
        }

        for task in difference.get_removed().get_tasks() {
            if let Some(t) = locked.get_mut(task.get_name()) {
                t.set_state(TaskState::STOPPING);
                t.set_environment(task.get_environment().to_owned().into_iter().collect());
                self.state.set_task(t);
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
        todo!();
    }
}

impl metal_grpc_rust::MetalService for MetalServiceHandler {
    fn update(
        &self,
        _m: grpc::RequestOptions,
        req: metal_grpc_rust::UpdateRequest,
    ) -> grpc::SingleResponse<metal_grpc_rust::UpdateResponse> {
        grpc::SingleResponse::completed(self.update(req))
    }

    fn diff(
        &self,
        _m: grpc::RequestOptions,
        req: metal_grpc_rust::UpdateRequest,
    ) -> grpc::SingleResponse<metal_grpc_rust::DiffResponse> {
        grpc::SingleResponse::completed(self.diff(req))
    }

    fn resolve(
        &self,
        _m: grpc::RequestOptions,
        req: metal_grpc_rust::ResolveRequest,
    ) -> grpc::SingleResponse<metal_grpc_rust::ResolveResponse> {
        grpc::SingleResponse::completed(self.resolve(req))
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
        let resp = service.update(update);

        assert_eq!(resp.get_diff_applied().get_added().get_tasks().len(), 1);

        // Now try to apply the same update again, and check diff
        let mut update = metal_grpc_rust::UpdateRequest::new();
        let mut t = Task::new();
        t.set_name(String::from("task_one"));
        update.mut_config().mut_tasks().push(t);
        let resp = service.update(update);

        assert_eq!(resp.get_diff_applied().get_added().get_tasks().len(), 0);

        // Now bring it down, should get removed
        let mut update = metal_grpc_rust::UpdateRequest::new();
        update.set_down(true);
        let mut t = Task::new();
        t.set_name(String::from("task_one"));
        update.mut_config().mut_tasks().push(t);
        let resp = service.update(update);

        assert_eq!(resp.get_diff_applied().get_removed().get_tasks().len(), 1);
        assert_eq!(resp.get_diff_applied().get_added().get_tasks().len(), 0);
    }
}

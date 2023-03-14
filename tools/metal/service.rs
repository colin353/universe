use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use metal_bus::{Configuration, DiffResponse, DiffType, Task, TaskRuntimeInfo, TaskState};
use state::{MetalStateError, MetalStateManager};

const RESOLUTION_TTL: u64 = 5;

#[derive(Clone)]
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
            .map(|t| (t.name.to_string(), t))
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
            let state = &runtime_state.state;

            // Update current task status in state
            if let Some(t) = _tasks.get_mut(&task_name) {
                t.runtime_info = runtime_state.clone();
                self.state.set_task(t).unwrap(); // Don't unwrap??
            }

            match state {
                TaskState::Success | TaskState::Stopped | TaskState::Failed => {
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
    for task in &desired.tasks {
        if let Some(current_task) = current.get(&task.name) {
            if down {
                response.removed.tasks.push(task.clone());
            } else {
                let difference = diff::diff_task(current_task, task);
                if difference.kind == DiffType::None {
                    response.added.tasks.push(task.clone());
                }
            }
        } else {
            if !down {
                response.added.tasks.push(task.clone());
            }
        }
    }
    response
}

impl metal_bus::MetalServiceHandler for MetalServiceHandler {
    fn update(
        &self,
        req: metal_bus::UpdateRequest,
    ) -> Result<metal_bus::UpdateResponse, bus::BusRpcError> {
        let difference: DiffResponse;
        {
            let mut locked = self.0.tasks.write().unwrap();
            difference = compute_diff(&locked, &req.config, req.down);
            for task in &difference.added.tasks {
                if let Some(t) = locked.get_mut(&task.name) {
                    t.binary = task.binary.clone();
                    t.environment = task.environment.clone();
                    t.arguments = task.arguments.clone();
                    self.0.state.set_task(t).unwrap();
                } else {
                    locked.insert(task.name.to_owned(), task.to_owned());
                    self.0.state.set_task(&task).unwrap();
                }
            }

            for task in &difference.removed.tasks {
                if let Some(t) = locked.get_mut(&task.name) {
                    t.environment = task.environment.to_owned().into_iter().collect();
                    self.0.state.set_task(t).unwrap();
                }
            }
        }

        let mut out = metal_bus::UpdateResponse::new();

        // Try to actually enact the difference using the monitor
        match self.0.monitor.execute(&difference) {
            Ok(_) => out.success = true,
            Err(e) => out.error_message = format!("failed to enact diff: {:?}", e),
        }

        out.diff_applied = difference;
        Ok(out)
    }

    fn diff(
        &self,
        req: metal_bus::UpdateRequest,
    ) -> Result<metal_bus::DiffResponse, bus::BusRpcError> {
        let locked = self.0.tasks.read().unwrap();
        Ok(compute_diff(&locked, &req.config, req.down))
    }

    fn resolve(
        &self,
        req: metal_bus::ResolveRequest,
    ) -> Result<metal_bus::ResolveResponse, bus::BusRpcError> {
        let mut response = metal_bus::ResolveResponse::new();

        // Try to resolve as a task + binding name
        if let Some(pos) = req.service_name.rfind('.') {
            let task_name = &req.service_name[..pos];
            let binding_name = &req.service_name[pos + 1..];
            if !task_name.is_empty() {
                let locked = self.0.tasks.read().unwrap();
                if let Some(t) = locked.get(task_name) {
                    for binding in &t.runtime_info.services {
                        if binding.service_name == binding_name {
                            let mut endpoint = metal_bus::Endpoint::new();
                            endpoint.ip_address = t.runtime_info.ip_address.to_owned();
                            endpoint.port = binding.port;
                            response.endpoints.push(endpoint);
                        }
                    }
                }
            }
        }

        Ok(response)
    }

    fn status(
        &self,
        req: metal_bus::StatusRequest,
    ) -> Result<metal_bus::StatusResponse, bus::BusRpcError> {
        let mut response = metal_bus::StatusResponse::new();
        for task in self.0.state.all_tasks().unwrap() {
            if task.name.starts_with(&req.selector) {
                response.tasks.push(task);
            }
        }
        Ok(response)
    }

    fn get_logs(
        &self,
        req: metal_bus::GetLogsRequest,
    ) -> Result<metal_bus::GetLogsResponse, bus::BusRpcError> {
        let mut response = metal_bus::GetLogsResponse::new();
        for log in self.0.monitor.get_logs(&req.resource_name).unwrap() {
            response.logs.push(log);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use metal_bus::MetalServiceHandler as _;
    use state::FakeState;

    #[test]
    fn test_simple_setup() {
        let state = Arc::new(FakeState::new());
        let monitor = Arc::new(core::FakeMonitor::new());
        let service = MetalServiceHandler::new(state, monitor).unwrap();

        let mut update = metal_bus::UpdateRequest::new();
        let mut t = Task::new();
        t.name = String::from("task_one");
        update.config.tasks.push(t);
        let resp = service.update(update).unwrap();

        assert_eq!(resp.diff_applied.added.tasks.len(), 1);

        // Now try to apply the same update again, and check diff
        let mut update = metal_bus::UpdateRequest::new();
        let mut t = Task::new();
        t.name = String::from("task_one");
        update.config.tasks.push(t);
        let resp = service.update(update).unwrap();

        assert_eq!(resp.diff_applied.added.tasks.len(), 0);

        // Now bring it down, should get removed
        let mut update = metal_bus::UpdateRequest::new();
        update.down = true;
        let mut t = Task::new();
        t.name = String::from("task_one");
        update.config.tasks.push(t);
        let resp = service.update(update).unwrap();

        assert_eq!(resp.diff_applied.removed.tasks.len(), 1);
        assert_eq!(resp.diff_applied.added.tasks.len(), 0);
    }
}

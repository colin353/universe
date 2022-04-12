use metal_grpc_rust::{DiffResponse, Task, TaskRuntimeInfo};

#[derive(Debug)]
pub enum MetalMonitorError {
    InvalidBinaryFormat(String),
    FailedToStartTask,
    FailedToCreateDirectories,
    PortSpaceExhausted,
    FailedToKillProcess,
    ConcurrencyError,
}

pub trait Coordinator: Send + Sync {
    fn report_tasks(&self, status: Vec<(String, TaskRuntimeInfo)>);
}

pub trait Monitor: Send + Sync {
    fn execute(&self, diff: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError>;
    fn monitor(&self) {}
}

pub struct FakeMonitor {}
impl FakeMonitor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Monitor for FakeMonitor {
    fn execute(&self, diff: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError> {
        Ok(Vec::new())
    }
}

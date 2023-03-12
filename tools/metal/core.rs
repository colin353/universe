use metal_bus::{DiffResponse, Logs, Task, TaskRuntimeInfo};

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
    fn report_tasks(&self, status: Vec<(String, TaskRuntimeInfo)>) -> Vec<String>;
}

pub trait Monitor: Send + Sync {
    fn execute(&self, diff: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError>;
    fn monitor(&self) {}
    fn restart_loop(&self) {}
    fn get_logs(&self, resource_name: &str) -> Result<Vec<Logs>, MetalMonitorError>;
}

pub struct FakeMonitor {}
impl FakeMonitor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Monitor for FakeMonitor {
    fn execute(&self, _: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError> {
        Ok(Vec::new())
    }

    fn get_logs(&self, _: &str) -> Result<Vec<Logs>, MetalMonitorError> {
        Ok(Vec::new())
    }
}

pub fn ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

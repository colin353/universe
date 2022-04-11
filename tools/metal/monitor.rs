use core::MetalMonitorError;
use metal_grpc_rust::{DiffResponse, Task, TaskRuntimeInfo};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

mod process;

pub struct PortAllocator {
    start: u16,
    end: u16,
    allocs: Mutex<Vec<bool>>,
}

impl PortAllocator {
    fn new(start: u16, end: u16) -> Self {
        Self {
            start,
            end,
            allocs: Mutex::new(Vec::new()),
        }
    }

    fn allocate_ports(&self, num_ports: usize) -> Result<Vec<u16>, MetalMonitorError> {
        let mut allocs = self.allocs.lock().unwrap();
        if allocs.len() as u16 >= self.end - self.start {
            // TODO: Try to find a port by reusing old ports
            return Err(MetalMonitorError::PortSpaceExhausted);
        }
        let mut out = Vec::new();
        for i in 0..num_ports {
            out.push(allocs.len() as u16 + self.start);
            allocs.push(true);
        }
        Ok(out)
    }

    fn deallocate_ports(&self, ports: &[u16]) {
        // TODO: actually deallocate ports so they can be reused
    }
}

pub struct MetalMonitor {
    tasks: RwLock<HashMap<String, RwLock<Task>>>,
    ip_address: std::net::IpAddr,
    port_allocator: PortAllocator,
    root_dir: std::path::PathBuf,
}

impl core::Monitor for MetalMonitor {
    fn execute(&self, diff: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError> {
        self.execute(diff)
    }
}

impl MetalMonitor {
    pub fn new(root_dir: std::path::PathBuf, ip_address: std::net::IpAddr) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            ip_address,
            port_allocator: PortAllocator::new(10000, 20000),
            root_dir,
        }
    }

    fn execute(&self, diff: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError> {
        let mut results = Vec::new();
        for added in diff.get_added().get_tasks() {
            // Update or create the task lock entry
            {
                let mut _taskmap = self.tasks.write().unwrap();
                let mut task = _taskmap
                    .entry(added.get_name().to_owned())
                    .or_insert_with(|| RwLock::new(added.clone()))
                    .write()
                    .unwrap();

                let mut added = added.clone();
                added.set_runtime_info(task.get_runtime_info().clone());
                *task = added
            }

            // Re-acquire the taskmap as a readlock, attempt to stop/start the task
            {
                let _taskmap = self.tasks.read().unwrap();
                let task_lock = _taskmap
                    .get(added.get_name())
                    .ok_or(MetalMonitorError::ConcurrencyError)?;

                let mut task = task_lock.write().unwrap();
                let runtime_info = self.stop_task(&task)?;
                task.set_runtime_info(runtime_info);
                let runtime_info = self.start_task(&task)?;
                task.set_runtime_info(runtime_info);

                results.push(task.clone());
            }
        }

        for removed in diff.get_removed().get_tasks() {
            let mut _taskmap = self.tasks.read().unwrap();
            let mut task = match _taskmap.get(removed.get_name()) {
                Some(t) => t.write().unwrap(),
                // No need to stop it if it doesn't exist
                None => continue,
            };

            let runtime_info = self.stop_task(&task)?;
            task.set_runtime_info(runtime_info);
            results.push(task.clone());
        }

        Ok(results)
    }
}

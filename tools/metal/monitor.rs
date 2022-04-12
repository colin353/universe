use core::{Coordinator, MetalMonitorError};
use metal_grpc_rust::{DiffResponse, RestartMode, Task, TaskRuntimeInfo, TaskState};

use std::collections::{HashMap, HashSet};
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
    coordinator: Mutex<Option<Arc<dyn Coordinator>>>,
    restart_queue: Mutex<std::collections::VecDeque<(String, u64)>>,
}

impl core::Monitor for MetalMonitor {
    fn execute(&self, diff: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError> {
        self.execute(diff)
    }

    fn monitor(&self) {
        self.monitor()
    }
}

impl MetalMonitor {
    pub fn new(root_dir: std::path::PathBuf, ip_address: std::net::IpAddr) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            ip_address,
            port_allocator: PortAllocator::new(10000, 20000),
            root_dir,
            coordinator: Mutex::new(None),
            restart_queue: Mutex::new(std::collections::VecDeque::new()),
        }
    }

    pub fn set_coordinator(&self, coordinator: Arc<dyn Coordinator>) {
        *self.coordinator.lock().unwrap() = Some(coordinator);
    }

    pub fn monitor(&self) {
        loop {
            let runtime_state = self.check_tasks();
            for (task_name, runtime_state) in &runtime_state {
                let mut restart_mode = RestartMode::ONE_SHOT;
                {
                    let _tasks = self.tasks.read().unwrap();
                    if let Some(t) = _tasks.get(task_name) {
                        let mut _t = t.write().unwrap();
                        restart_mode = _t.get_restart_mode();
                        _t.set_runtime_info(runtime_state.clone());
                    }
                }

                // If the task is no longer running, we may need to restart it, or else clean it up
                if runtime_state.get_state() != TaskState::RUNNING {
                    match restart_mode {
                        RestartMode::ONE_SHOT => {
                            // Tear down task
                        }
                        mode => {
                            if mode == RestartMode::ON_FAILURE
                                && runtime_state.get_state() == TaskState::SUCCESS
                            {
                                continue;
                            }

                            self.restart_queue.lock().unwrap().push_back((
                                task_name.clone(),
                                // Last stopped time + 5 seconds
                                runtime_state.get_last_stopped_time() + 5 * 1000 * 1000,
                            ));
                        }
                    }
                }
            }

            // Report task state to coordinator
            if !runtime_state.is_empty() {
                if let Some(c) = &*self.coordinator.lock().unwrap() {
                    c.report_tasks(runtime_state);
                }
            }

            // Possibly restart tasks
            let queue = self.restart_queue.lock().unwrap();
            let now = process::ts();
            for (task_name, timestamp) in queue.iter() {
                if *timestamp < now {
                    println!("should restart {}", task_name);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
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

use std::collections::HashMap;

use crate::{MetalMonitorError, MetalMonitorInner, PortAllocator};
use metal_grpc_rust::{ArgKind, ServiceAssignment, Task, TaskRuntimeInfo};

fn ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

impl MetalMonitorInner {
    fn ip_address(&self) -> Vec<u8> {
        match self.ip_address {
            std::net::IpAddr::V4(a) => a.octets().to_vec(),
            std::net::IpAddr::V6(a) => a.octets().to_vec(),
        }
    }

    fn resource_logs_dir(&self, name: &str) -> std::path::PathBuf {
        let mut out = self.root_dir.join("logs");
        for component in name.split(".") {
            out.push(component);
        }
        out
    }

    pub fn stop_task(&self, task: &Task) -> Result<TaskRuntimeInfo, MetalMonitorError> {
        if task.get_runtime_info().get_pid() == 0 {
            return Ok(task.get_runtime_info().clone());
        }

        match unsafe { libc::kill(task.get_runtime_info().get_pid() as i32, libc::SIGTERM) } {
            // If the process isn't found, that means we don't need to kill it
            libc::ESRCH => (),
            // 0 indicates success
            0 => (),
            // Something else happened
            _ => return Err(MetalMonitorError::FailedToKillProcess),
        }

        let mut runtime_info = task.get_runtime_info().clone();
        runtime_info.set_last_stopped_time(ts());
        runtime_info.set_exit_status(128 + libc::SIGTERM); // NOTE: traditional exit code for being killed

        Ok(runtime_info)
    }

    pub fn start_task(&self, task: &Task) -> Result<TaskRuntimeInfo, MetalMonitorError> {
        // Make sure that the appropriate parent directories are created
        let logs_dir = self.resource_logs_dir(task.get_name());
        std::fs::create_dir_all(&logs_dir)
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;

        if task.get_binary().get_path().is_empty() {
            return Err(MetalMonitorError::InvalidBinaryFormat(String::from(
                "only path-based binary resolution is implemented",
            )));
        }

        // Allocate all the service ports that we need
        let mut ports = HashMap::new();
        for arg in task.get_arguments() {
            if arg.get_kind() == ArgKind::PORT_ASSIGNMENT {
                ports.insert(arg.get_value().to_string(), 0);
            }
        }
        for env in task.get_environment() {
            if env.get_value().get_kind() == ArgKind::PORT_ASSIGNMENT {
                ports.insert(env.get_value().get_value().to_string(), 0);
            }
        }
        let mut allocated_ports = self.port_allocator.allocate_ports(ports.len())?;
        for (k, v) in ports.iter_mut() {
            *v = allocated_ports.pop().expect("should have enough ports");
        }

        let mut process = std::process::Command::new(task.get_binary().get_path());

        for arg in task.get_arguments() {
            if arg.get_kind() == ArgKind::PORT_ASSIGNMENT {
                process.arg(format!(
                    "{}",
                    ports
                        .get(arg.get_value())
                        .expect("must have allocated port!")
                ));
            } else {
                process.arg(arg.get_value());
            }
        }

        for env in task.get_environment() {
            let value = if env.get_value().get_kind() == ArgKind::PORT_ASSIGNMENT {
                format!(
                    "{}",
                    ports
                        .get(env.get_value().get_value())
                        .expect("must have allocated port!")
                )
            } else {
                env.get_value().get_value().to_string()
            };

            process.env(env.get_name(), value);
        }

        let start_time = ts();

        let stdout_file = std::fs::File::create(logs_dir.join(format!("STDOUT.{}", start_time)))
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;
        let stderr_file = std::fs::File::create(logs_dir.join(format!("STDERR.{}", start_time)))
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;

        let child = process
            .stdout(stdout_file)
            .stderr(stderr_file)
            .spawn()
            .map_err(|e| MetalMonitorError::FailedToStartTask)?;

        let mut runtime_info = TaskRuntimeInfo::new();
        runtime_info.set_pid(child.id());
        runtime_info.set_last_start_time(start_time);
        runtime_info.set_ip_address(self.ip_address());

        for (service_name, port) in ports {
            let mut assignment = ServiceAssignment::new();
            assignment.set_service_name(service_name);
            assignment.set_port(port as u32);
            runtime_info.mut_services().push(assignment);
        }

        Ok(runtime_info)
    }
}

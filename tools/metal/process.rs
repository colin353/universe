use std::collections::HashMap;

use crate::{MetalMonitor, MetalMonitorError, PortAllocator};
use metal_grpc_rust::{ArgKind, ServiceAssignment, Task, TaskRuntimeInfo, TaskState};

pub fn ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

impl MetalMonitor {
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

        let start_time = ts();

        // A wrapper script is necessary to track the stop time and exit code of the binary.
        let mut process = std::process::Command::new("/bin/sh");
        process.arg("-c");
        process.arg(format!(
            "$@; echo $? > {}; date +%s > {};",
            logs_dir
                .join(format!("EXIT_STATUS.{}", start_time))
                .to_string_lossy(),
            logs_dir
                .join(format!("EXIT_TIME.{}", start_time))
                .to_string_lossy()
        ));
        process.arg("--");

        // All remaining arguments are the "real" command
        process.arg(task.get_binary().get_path());

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

        let stdout_file = std::fs::File::create(logs_dir.join(format!("STDOUT.{}", start_time)))
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;
        let stderr_file = std::fs::File::create(logs_dir.join(format!("STDERR.{}", start_time)))
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;

        let mut child = process
            .stdout(stdout_file)
            .stderr(stderr_file)
            .spawn()
            .map_err(|e| MetalMonitorError::FailedToStartTask)?;

        let mut runtime_info = TaskRuntimeInfo::new();
        runtime_info.set_state(TaskState::RUNNING);
        runtime_info.set_pid(child.id());
        runtime_info.set_last_start_time(start_time);
        runtime_info.set_ip_address(self.ip_address());

        std::thread::spawn(move || {
            child.wait();
        });

        for (service_name, port) in ports {
            let mut assignment = ServiceAssignment::new();
            assignment.set_service_name(service_name);
            assignment.set_port(port as u32);
            runtime_info.mut_services().push(assignment);
        }

        Ok(runtime_info)
    }

    pub fn check_tasks(&self) -> Vec<(String, TaskRuntimeInfo)> {
        let all_tasks: Vec<_> = self
            .tasks
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.read().unwrap().get_runtime_info().to_owned()))
            .collect();

        let mut to_update = Vec::new();
        for (task_name, mut runtime_info) in all_tasks {
            // Skip if the process hasn't been scheduled yet
            if runtime_info.get_pid() == 0 {
                continue;
            }

            let prev_state = runtime_info.get_state();
            let new_state = match get_proc_state(runtime_info.get_pid()) {
                Some(ProcessState::Running) => TaskState::RUNNING,
                Some(ProcessState::Exited(status)) => {
                    runtime_info.set_exit_status(status);
                    if status == 0 {
                        TaskState::SUCCESS
                    } else {
                        TaskState::FAILED
                    }
                }
                None => TaskState::UNKNOWN,
            };
            runtime_info.set_state(new_state);

            if runtime_info.get_state() != TaskState::RUNNING {
                // The process has newly ended. Let's look up the termination timestamp
                // and update the runtime info with that as well.
                let resource_dir = self.resource_logs_dir(&task_name);

                if let Ok(s) = std::fs::read_to_string(resource_dir.join(format!(
                    "EXIT_STATUS.{}",
                    runtime_info.get_last_start_time()
                ))) {
                    if let Ok(i) = s.trim().parse() {
                        runtime_info.set_exit_status(i);
                        if i == 0 {
                            runtime_info.set_state(TaskState::SUCCESS);
                        } else {
                            runtime_info.set_state(TaskState::FAILED);
                        }
                    }
                }

                if let Ok(s) = std::fs::read_to_string(
                    resource_dir.join(format!("EXIT_TIME.{}", runtime_info.get_last_start_time())),
                ) {
                    if let Ok(i) = s.trim().parse::<u64>() {
                        runtime_info.set_last_stopped_time(i * 1000 * 1000);
                    }
                }
            }

            // Don't report known state changes
            if prev_state == runtime_info.get_state() {
                continue;
            }

            to_update.push((task_name, runtime_info));
        }

        to_update
    }
}

#[derive(Debug)]
enum ProcessState {
    Running,
    Exited(i32),
}

fn get_proc_state(pid: u32) -> Option<ProcessState> {
    let data = match std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        Ok(d) => d,
        Err(_) => return None,
    };

    let exit_code: i32 = match &data[data.rfind(' ')?..].parse() {
        Ok(i) => *i,
        Err(_) => return None,
    };

    let start = data.find(')')?;

    Some(match &data[start + 1..start + 2] {
        "R" | "P" | "K" | "W" | "t" | "S" | "D" => ProcessState::Running,
        "T" | "X" | "x" => ProcessState::Exited(exit_code),
        _ => return None,
    })
}

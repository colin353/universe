use std::collections::HashMap;

use crate::{MetalMonitor, MetalMonitorError, PortAllocator};
use core::ts;
use metal_bus::{ArgKind, ServiceAssignment, Task, TaskRuntimeInfo, TaskState};

impl MetalMonitor {
    fn ip_address(&self) -> Vec<u8> {
        match self.ip_address {
            std::net::IpAddr::V4(a) => a.octets().to_vec(),
            std::net::IpAddr::V6(a) => a.octets().to_vec(),
        }
    }

    pub fn resource_logs_dir(&self, name: &str) -> std::path::PathBuf {
        let mut out = self.root_dir.join("logs");
        for component in name.split(".") {
            out.push(component);
        }
        out
    }

    pub fn stop_task(&self, task: &Task) -> Result<TaskRuntimeInfo, MetalMonitorError> {
        if task.runtime_info.pid == 0 {
            return Ok(task.runtime_info.clone());
        }

        match unsafe { libc::kill(task.runtime_info.pid as i32, libc::SIGTERM) } {
            // If the process isn't found, that means we don't need to kill it
            libc::ESRCH => (),
            // 0 indicates success
            0 => (),
            // Process isn't running, that's OK
            _ => (),
        }

        let mut runtime_info = task.runtime_info.clone();
        runtime_info.last_stopped_time = ts();
        runtime_info.exit_status = 128 + libc::SIGTERM; // NOTE: traditional exit code for being killed

        Ok(runtime_info)
    }

    pub fn start_task(&self, task: &Task) -> Result<TaskRuntimeInfo, MetalMonitorError> {
        // Make sure that the appropriate parent directories are created
        let logs_dir = self.resource_logs_dir(&task.name);
        std::fs::create_dir_all(&logs_dir)
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;

        if task.binary.path.is_empty() {
            return Err(MetalMonitorError::InvalidBinaryFormat(String::from(
                "only path-based binary resolution is implemented",
            )));
        }

        // Allocate all the service ports that we need
        let mut ports = HashMap::new();
        for arg in &task.arguments {
            if arg.kind == ArgKind::PortAssignment {
                ports.insert(arg.value.to_string(), 0);
            }
        }
        for env in &task.environment {
            if env.value.kind == ArgKind::PortAssignment {
                ports.insert(env.value.value.to_string(), 0);
            }
        }
        let mut allocated_ports = self.port_allocator.allocate_ports(ports.len())?;
        for (_, v) in ports.iter_mut() {
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
        process.arg(&task.binary.path);

        for arg in &task.arguments {
            if arg.kind == ArgKind::PortAssignment {
                process.arg(format!(
                    "{}",
                    ports.get(&arg.value).expect("must have allocated port!")
                ));
            } else {
                process.arg(&arg.value);
            }
        }

        for env in &task.environment {
            let value = if env.value.kind == ArgKind::PortAssignment {
                format!(
                    "{}",
                    ports
                        .get(&env.value.value)
                        .expect("must have allocated port!")
                )
            } else {
                env.value.value.to_string()
            };

            process.env(&env.name, value);
        }

        let stdout_file = std::fs::File::create(logs_dir.join(format!("STDOUT.{}", start_time)))
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;
        let stderr_file = std::fs::File::create(logs_dir.join(format!("STDERR.{}", start_time)))
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;

        let mut child = process
            .stdout(stdout_file)
            .stderr(stderr_file)
            .spawn()
            .map_err(|_| MetalMonitorError::FailedToStartTask)?;

        let mut runtime_info = TaskRuntimeInfo::new();
        runtime_info.state = TaskState::Running;
        runtime_info.pid = child.id();
        runtime_info.last_start_time = start_time;
        runtime_info.ip_address = self.ip_address();

        std::thread::spawn(move || {
            child.wait();
        });

        for (service_name, port) in ports {
            let mut assignment = ServiceAssignment::new();
            assignment.service_name = service_name;
            assignment.port = port as u32;
            runtime_info.services.push(assignment);
        }

        Ok(runtime_info)
    }

    pub fn check_tasks(&self) -> Vec<(String, TaskRuntimeInfo)> {
        let all_tasks: Vec<_> = self
            .tasks
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.read().unwrap().runtime_info.to_owned()))
            .collect();

        let mut to_update = Vec::new();
        for (task_name, mut runtime_info) in all_tasks {
            // Skip if the process hasn't been scheduled yet
            if runtime_info.pid == 0 {
                continue;
            }

            let prev_state = runtime_info.state;

            // If we already know this task is waiting for restart, don't probe
            if prev_state == TaskState::Restarting {
                continue;
            }

            let new_state = match get_proc_state(runtime_info.pid) {
                Some(ProcessState::Running) => TaskState::Running,
                Some(ProcessState::Exited(status)) => {
                    runtime_info.exit_status = status;
                    if status == 0 {
                        TaskState::Success
                    } else {
                        TaskState::Failed
                    }
                }
                None => TaskState::Unknown,
            };
            runtime_info.state = new_state;

            if runtime_info.state != TaskState::Running {
                // The process has newly ended. Let's look up the termination timestamp
                // and update the runtime info with that as well.
                let resource_dir = self.resource_logs_dir(&task_name);

                if let Ok(s) = std::fs::read_to_string(
                    resource_dir.join(format!("EXIT_STATUS.{}", runtime_info.last_start_time)),
                ) {
                    if let Ok(i) = s.trim().parse() {
                        runtime_info.exit_status = i;
                        if i == 0 {
                            runtime_info.state = TaskState::Success;
                        } else {
                            runtime_info.state = TaskState::Failed;
                        }
                    }
                }

                if let Ok(s) = std::fs::read_to_string(
                    resource_dir.join(format!("EXIT_TIME.{}", runtime_info.last_start_time)),
                ) {
                    if let Ok(i) = s.trim().parse::<u64>() {
                        runtime_info.last_stopped_time = i * 1000 * 1000;
                    }
                }
            }

            // Don't report known state changes
            if prev_state == runtime_info.state {
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

#[cfg(not(target_os = "macos"))]
fn get_proc_state(pid: u32) -> Option<ProcessState> {
    let data = match std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        Ok(d) => d,
        Err(_) => return None,
    };

    let exit_code: i32 = match data[data.rfind(' ')?..].trim().parse() {
        Ok(i) => i,
        Err(_) => return None,
    };

    let start = data.find(')')?;
    Some(match &data[start + 2..start + 3] {
        "R" | "P" | "K" | "W" | "t" | "S" | "D" => ProcessState::Running,
        "T" | "X" | "x" => ProcessState::Exited(exit_code),
        _ => return None,
    })
}

// NOTE: this was written by chatGPT so I have no clue if it's right,
// but it seems to work
#[cfg(target_os = "macos")]
fn get_proc_state(pid: u32) -> Option<ProcessState> {
    let output = match std::process::Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "state="])
        .output()
    {
        Ok(output) => output,
        Err(_) => {
            return None;
        }
    };

    if !output.status.success() {
        // Exited? Harvest exit code somehow?
        return Some(ProcessState::Exited(1));
    }

    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(_) => return None,
    };

    let mut parts = stdout.trim().split_whitespace();
    let state = match parts.next().and_then(|s| s.chars().next()) {
        Some('R') => ProcessState::Running,
        Some('S') => ProcessState::Running,
        Some('T') => ProcessState::Running,
        Some('Z') => {
            // Exited? Harvest exit code? Just say exit 1
            return Some(ProcessState::Exited(1));
        }
        x => {
            return None;
        }
    };

    Some(state)
}

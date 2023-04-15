use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

use crate::{MetalMonitor, PortAllocator};
use core::{ts, MetalMonitorError};
use metal_bus::{ArgKind, ServiceAssignment, Task, TaskRuntimeInfo, TaskState};

impl MetalMonitor {
    fn ip_address(&self) -> Vec<u8> {
        match self.0.ip_address {
            std::net::IpAddr::V4(a) => a.octets().to_vec(),
            std::net::IpAddr::V6(a) => a.octets().to_vec(),
        }
    }

    pub fn resource_logs_dir(&self, name: &str) -> std::path::PathBuf {
        let mut out = self.0.root_dir.join("logs");
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

    pub async fn download_and_start_task(&self, task: &Task) {
        let mut task = task.clone();
        if task.binary.url.starts_with("rainbow://") {
            let (binary, tag) = match rainbow::parse(&task.binary.url[10..]) {
                Ok(b) => b,
                Err(_) => {
                    let mut runtime_info = TaskRuntimeInfo::new();
                    runtime_info.state = TaskState::Failed;
                    self.write_log(
                        &task.name,
                        &format!("failed to parse rainbow binary spec {:?}", task.binary.url),
                    )
                    .unwrap();
                    if let Some(c) = &*self.0.coordinator.lock().unwrap() {
                        c.report_tasks(vec![(task.name.to_owned(), runtime_info)]);
                    }
                    return;
                }
            };

            task.binary.url = match rainbow::async_resolve(binary, tag).await {
                Some(b) => {
                    // Detect tarfiles
                    task.binary.is_tar = b.ends_with(".tar");
                    b
                }
                None => {
                    let mut runtime_info = TaskRuntimeInfo::new();
                    runtime_info.state = TaskState::Failed;
                    self.write_log(
                        &task.name,
                        &format!("failed to resolve rainbow binary {binary}:{tag}"),
                    )
                    .unwrap();
                    if let Some(c) = &*self.0.coordinator.lock().unwrap() {
                        c.report_tasks(vec![(task.name.to_owned(), runtime_info)]);
                    }
                    return;
                }
            };

            println!("resolved to {}", task.binary.url);
        }

        if let Err(e) = self.download_binary(&task.binary).await {
            // Failed to download the binary, update status as failed
            let mut runtime_info = TaskRuntimeInfo::new();
            runtime_info.state = TaskState::Failed;
            self.write_log(&task.name, &format!("{e:#?}")).unwrap();
            if let Some(c) = &*self.0.coordinator.lock().unwrap() {
                c.report_tasks(vec![(task.name.to_owned(), runtime_info)]);
            }
            return;
        }

        match self.start_task(&task) {
            Ok(runtime_info) => {
                // Update coordinator with runtime info
                if let Some(c) = &*self.0.coordinator.lock().unwrap() {
                    c.report_tasks(vec![(task.name.to_owned(), runtime_info)]);
                }
            }
            Err(e) => {
                // Indicate failure
                let mut runtime_info = TaskRuntimeInfo::new();
                runtime_info.state = TaskState::Failed;
                self.write_log(&task.name, &format!("{e:#?}")).unwrap();
                if let Some(c) = &*self.0.coordinator.lock().unwrap() {
                    c.report_tasks(vec![(task.name.to_owned(), runtime_info)]);
                }
            }
        }
    }

    pub fn start_task(&self, task: &Task) -> Result<TaskRuntimeInfo, MetalMonitorError> {
        // Make sure that the appropriate parent directories are created
        let logs_dir = self.resource_logs_dir(&task.name);
        std::fs::create_dir_all(&logs_dir)
            .map_err(|_| MetalMonitorError::FailedToCreateDirectories)?;

        if task.binary.path.is_empty() && task.binary.url.is_empty() {
            return Err(MetalMonitorError::InvalidBinaryFormat(String::from(
                "only URL-based and path-based binary resolution is implemented",
            )));
        }

        // Download the file's url if necessary
        let mut binary_path = if !task.binary.path.is_empty() {
            std::path::PathBuf::from(&task.binary.path)
        } else {
            let is_rainbow = task.binary.url.starts_with("rainbow://");

            let path = self.binary_cache_path_for_url(&task.binary.url);
            if is_rainbow || !path.exists() {
                // We need to download the binary before we can start the task. Let's schedule
                // the task to be downloaded and set the current state as Preparing.
                let _t = task.clone();
                let _self = self.clone();
                tokio::spawn(async move {
                    _self.download_and_start_task(&_t).await;
                });

                let mut runtime_info = TaskRuntimeInfo::new();
                runtime_info.state = TaskState::Preparing;
                return Ok(runtime_info);
            }
            path
        };

        // If the binary is a tarball, the path is not the executable itself, but the root dir of
        // the tarball. In that case, we need to find the first executable binary in that dir
        // and execute it with cwd as the tarball root.
        let mut cwd = None;
        if task.binary.is_tar {
            cwd = Some(binary_path.clone());

            for entry in std::fs::read_dir(&binary_path).map_err(|e| {
                return MetalMonitorError::InvalidBinaryFormat(String::from(
                    "tarball directory not valid",
                ));
            })? {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                match entry.file_type() {
                    Ok(t) => {
                        if t.is_dir() {
                            continue;
                        }
                    }
                    Err(_) => continue,
                };

                if let Ok(metadata) = entry.metadata() {
                    if metadata.permissions().mode() & 0o111 != 0 {
                        // It's executable, use this path
                        binary_path = entry.path();
                    }
                }
            }
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
        let mut allocated_ports = self.0.port_allocator.allocate_ports(ports.len())?;
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

        if let Some(cwd) = cwd {
            println!("using cwd = {cwd:?}");
            process.current_dir(cwd);
        }

        process.arg("--");

        // All remaining arguments are the "real" command
        process.arg(&binary_path);

        for arg in &task.arguments {
            match arg.kind {
                ArgKind::PortAssignment => {
                    process.arg(format!(
                        "{}",
                        ports.get(&arg.value).expect("must have allocated port!")
                    ));
                }
                ArgKind::Secret => {
                    let secret_value = std::fs::read_to_string(&arg.value).map_err(|_| {
                        MetalMonitorError::InaccessibleSecret(arg.value.to_string())
                    })?;
                    process.arg(secret_value);
                }
                _ => {
                    process.arg(&arg.value);
                }
            }
        }

        for env in &task.environment {
            let value = match env.value.kind {
                ArgKind::PortAssignment => {
                    format!(
                        "{}",
                        ports
                            .get(&env.value.value)
                            .expect("must have allocated port!")
                    )
                }
                ArgKind::Secret => std::fs::read_to_string(&env.value.value).map_err(|_| {
                    MetalMonitorError::InaccessibleSecret(env.value.value.to_string())
                })?,
                _ => env.value.value.to_string(),
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
            child.wait().unwrap();
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
            .0
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

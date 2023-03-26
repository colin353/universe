use core::{Coordinator, MetalMonitorError};
use metal_bus::{DiffResponse, Logs, RestartMode, Task, TaskState};

use futures::future::Shared;
use futures::{Future, FutureExt};
use hyper::body::HttpBody;

use std::collections::HashMap;
use std::io::{Read, Seek, Write};
use std::os::unix::fs::PermissionsExt;
use std::pin::Pin;
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
        for _ in 0..num_ports {
            out.push(allocs.len() as u16 + self.start);
            allocs.push(true);
        }
        Ok(out)
    }

    fn deallocate_ports(&self, _ports: &[u16]) {
        // TODO: actually deallocate ports so they can be reused
    }
}

#[derive(Clone)]
pub struct MetalMonitor(Arc<MetalMonitorInner>);

pub struct MetalMonitorInner {
    tasks: RwLock<HashMap<String, RwLock<Task>>>,
    ip_address: std::net::IpAddr,
    port_allocator: PortAllocator,
    root_dir: std::path::PathBuf,
    coordinator: Mutex<Option<Arc<dyn Coordinator>>>,
    restart_queue: Mutex<Vec<(String, u64)>>,
    restart_accumulator: Mutex<HashMap<String, (u64, u64)>>,
    downloads: Mutex<
        HashMap<
            String,
            Shared<
                Pin<Box<dyn Future<Output = Result<std::path::PathBuf, MetalMonitorError>> + Send>>,
            >,
        >,
    >,
}

impl core::Monitor for MetalMonitor {
    fn execute(&self, diff: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError> {
        self.execute(diff)
    }

    fn monitor(&self) {
        self.monitor()
    }

    fn restart_loop(&self) {
        self.restart_loop()
    }

    fn get_logs(&self, resource_name: &str) -> Result<Vec<Logs>, MetalMonitorError> {
        let logs_dir = self.resource_logs_dir(resource_name);
        let iter = match std::fs::read_dir(logs_dir) {
            Ok(i) => i,
            Err(_) => return Ok(Vec::new()),
        };

        let mut log_files = HashMap::new();
        for entry in iter {
            let entry = match entry {
                Ok(i) => i,
                Err(_) => continue,
            };

            let start_time: u64 = match entry
                .path()
                .extension()
                .unwrap_or(std::ffi::OsStr::new(""))
                .to_string_lossy()
                .parse()
            {
                Ok(t) => t,
                Err(_) => continue,
            };

            let files = log_files.entry(start_time).or_insert(Vec::new());
            files.push(entry.path());
        }

        let mut sorted_logs: Vec<_> = log_files.into_iter().collect();
        sorted_logs.sort_by_key(|(t, _)| *t);

        let mut remaining_bytes: i64 = 1_048_576;
        let mut out = Vec::new();
        for (t, paths) in sorted_logs.iter().rev() {
            if remaining_bytes <= 0 {
                break;
            }

            let mut log_entry = Logs::new();
            log_entry.start_time = *t;
            for path in paths {
                let stem = match path.file_stem().map(|s| s.to_string_lossy()) {
                    Some(s) => s,
                    None => continue,
                };

                match stem.as_ref() {
                    "EXIT_TIME" => match std::fs::read_to_string(path) {
                        Ok(c) => {
                            if let Ok(i) = c.trim().parse::<u64>() {
                                log_entry.end_time = i * 1_000_000;
                            }
                        }
                        Err(_) => continue,
                    },
                    "EXIT_STATUS" => match std::fs::read_to_string(path) {
                        Ok(c) => {
                            if let Ok(i) = c.trim().parse() {
                                log_entry.exit_status = i;
                            }
                        }
                        Err(_) => continue,
                    },
                    s @ "STDOUT" | s @ "STDERR" => match std::fs::File::open(path) {
                        Ok(mut f) => {
                            if let Err(_) = f.seek(std::io::SeekFrom::End(-remaining_bytes)) {
                                f.seek(std::io::SeekFrom::Start(0))
                                    .expect("failed to seek!");
                            }

                            let mut log_data = String::new();
                            if let Ok(b) = f.read_to_string(&mut log_data) {
                                remaining_bytes = remaining_bytes - (b as i64);

                                if s == "STDOUT" {
                                    log_entry.stdout = log_data;
                                } else {
                                    log_entry.stderr = log_data;
                                }
                            }
                        }
                        Err(_) => continue,
                    },
                    _ => continue,
                }
            }

            out.push(log_entry);
        }

        // Reverse the order of the log entries, so oldest is first
        out.reverse();

        Ok(out)
    }
}

impl MetalMonitor {
    pub fn new(root_dir: std::path::PathBuf, ip_address: std::net::IpAddr) -> Self {
        Self(Arc::new(MetalMonitorInner {
            tasks: RwLock::new(HashMap::new()),
            ip_address,
            port_allocator: PortAllocator::new(10000, 20000),
            root_dir,
            coordinator: Mutex::new(None),
            restart_queue: Mutex::new(Vec::new()),
            restart_accumulator: Mutex::new(HashMap::new()),
            downloads: Mutex::new(HashMap::new()),
        }))
    }

    pub fn write_log(&self, resource_name: &str, message: &str) -> std::io::Result<()> {
        let logs_dir = self.resource_logs_dir(resource_name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(logs_dir.join(format!("STDERR.{}", core::ts())))?;

        file.write_all("\n".as_bytes())?;
        file.write_all(message.as_bytes())?;
        file.write_all("\n".as_bytes())?;
        Ok(())
    }

    pub fn binary_cache_path_for_url(&self, url: &str) -> std::path::PathBuf {
        let mut h = sha256::Sha256::new();
        h.absorb(url.as_bytes());
        let filename = h
            .finish()
            .iter()
            .map(|b| format!("{b:x}"))
            .collect::<Vec<_>>()
            .join("");
        self.0.root_dir.join("binaries").join(filename)
    }

    pub async fn download_binary(
        &self,
        url: &str,
    ) -> Result<std::path::PathBuf, MetalMonitorError> {
        let maybe_fut = {
            let mut _downloads = self.0.downloads.lock().unwrap();
            _downloads.get(url).map(|f| f.clone())
        };
        if let Some(fut) = maybe_fut {
            return fut.await;
        }

        let _url = url.to_string();
        let _self = self.clone();
        let fut = async move { _self._download_binary(_url).await }
            .boxed()
            .shared();

        {
            let mut _downloads = self.0.downloads.lock().unwrap();
            _downloads.insert(url.to_string(), fut.clone());
        }

        fut.await
    }

    async fn _download_binary(&self, url: String) -> Result<std::path::PathBuf, MetalMonitorError> {
        if url.is_empty() {
            return Err(MetalMonitorError::FailedToDownloadBinary(String::from(
                "url is empty",
            )));
        }

        std::fs::create_dir_all(self.0.root_dir.join("binaries")).map_err(|e| {
            MetalMonitorError::FailedToDownloadBinary(String::from(
                "failed to create binaries directory: {e:?}",
            ))
        });

        let out_path = self.binary_cache_path_for_url(&url);
        if out_path.exists() {
            return Ok(out_path);
        }

        let uri: hyper::Uri = url.parse().map_err(|_| {
            MetalMonitorError::FailedToDownloadBinary(format!("invalid url: {url:?}"))
        })?;

        let https = hyper_tls::HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        let mut res = client.get(uri).await.map_err(|e| {
            MetalMonitorError::FailedToDownloadBinary(format!("download request failed: {e:#?}"))
        })?;

        if !res.status().is_success() {
            return Err(MetalMonitorError::FailedToDownloadBinary(format!(
                "download failed with status code {} {:?}",
                res.status().as_str(),
                res.status().canonical_reason()
            )));
        }

        let mut tmp_path = out_path.clone();
        tmp_path.set_extension("temp");

        let mut f = std::fs::File::create(&tmp_path).map_err(|_| {
            MetalMonitorError::FailedToDownloadBinary(format!("failed to create temp file"))
        })?;
        while let Some(next) = res.data().await {
            let chunk = next.map_err(|e| {
                MetalMonitorError::FailedToDownloadBinary(format!("invalid url: {e:#?}"))
            })?;
            f.write_all(&chunk).map_err(|e| {
                MetalMonitorError::FailedToDownloadBinary(format!("failed to write to disk: {e:?}"))
            })?;
        }

        std::fs::rename(&tmp_path, &out_path).map_err(|e| {
            MetalMonitorError::FailedToDownloadBinary(format!(
                "failed to rename downloaded binary {e:?}"
            ))
        })?;
        let perm = std::fs::Permissions::from_mode(0o777);
        std::fs::set_permissions(&out_path, perm).map_err(|_| {
            MetalMonitorError::FailedToDownloadBinary(format!(
                "failed to set permissions on binary"
            ))
        })?;
        Ok(out_path)
    }

    pub fn set_coordinator(&self, coordinator: Arc<dyn Coordinator>) {
        *self.0.coordinator.lock().unwrap() = Some(coordinator);
    }

    pub fn queue_restart(&self, task_name: &str) {
        // Check the accumulator to see what the delay should be
        let target_start_time = {
            let now = core::ts();
            let mut acc = self.0.restart_accumulator.lock().unwrap();
            let delay = if let Some((delay, ts)) = acc.get(task_name) {
                let decayed_delay = (*delay as f64
                    * 2f64.powf(1.0 - ((now - ts) as f64) / (60_000_000 as f64)))
                    as u64;
                std::cmp::min(30_000_000, std::cmp::max(1_000_000, decayed_delay))
            } else {
                1_000_000
            };
            acc.insert(task_name.to_string(), (delay, now));
            delay + now
        };

        let mut queue = self.0.restart_queue.lock().unwrap();
        queue.push((task_name.to_owned(), target_start_time));
        queue.sort_by_key(|(_, t)| *t);
        queue.dedup_by_key(|(n, _)| n.clone());
    }

    pub fn monitor(&self) {
        loop {
            let mut runtime_state = self.check_tasks();
            for (task_name, runtime_state) in &mut runtime_state {
                let mut restart_mode = RestartMode::OneShot;
                {
                    let _tasks = self.0.tasks.read().unwrap();
                    if let Some(t) = _tasks.get(task_name) {
                        let mut _t = t.write().unwrap();
                        restart_mode = _t.restart_mode;
                        _t.runtime_info = runtime_state.clone();
                    }
                }

                // If the task is no longer running, we may need to restart it, or else clean it up
                if runtime_state.state != TaskState::Running
                    && runtime_state.state != TaskState::Preparing
                    && runtime_state.state != TaskState::Restarting
                {
                    match restart_mode {
                        RestartMode::OneShot => {
                            // No need to restart
                        }
                        mode => {
                            if mode == RestartMode::OnFailure
                                && runtime_state.state == TaskState::Success
                            {
                                continue;
                            }

                            // Mark task state as restarting
                            match self.0.tasks.read().unwrap().get(task_name) {
                                Some(t) => {
                                    t.write().unwrap().runtime_info.state = TaskState::Restarting
                                }
                                _ => (),
                            }
                            runtime_state.state = TaskState::Restarting;

                            self.queue_restart(&task_name);
                        }
                    }
                }
            }

            // Report task state to coordinator
            let mut to_clean_up = Vec::new();
            if !runtime_state.is_empty() {
                if let Some(c) = &*self.0.coordinator.lock().unwrap() {
                    to_clean_up = c.report_tasks(runtime_state);
                }
            }

            // Clean up removed tasks
            if !to_clean_up.is_empty() {
                let mut _tasks = self.0.tasks.write().unwrap();
                for task_name in &to_clean_up {
                    _tasks.remove(task_name);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    pub fn restart_loop(&self) {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));

            // Possibly restart tasks
            let mut queue = self.0.restart_queue.lock().unwrap();
            let now = core::ts();
            let mut to_take = 0;
            for (_, timestamp) in queue.iter() {
                if *timestamp < now {
                    to_take += 1;
                } else {
                    break;
                }
            }
            if to_take == 0 {
                continue;
            }

            let mut to_restart = queue.split_off(to_take);
            std::mem::swap(&mut to_restart, &mut *queue);
            for (task_name, _) in to_restart {
                let task = match self.0.tasks.read().unwrap().get(&task_name) {
                    Some(t) => t.read().unwrap().clone(),
                    None => {
                        // Task was unscheduled before it could be restarted
                        continue;
                    }
                };

                if let Ok(runtime_info) = self.start_task(&task) {
                    match self.0.tasks.read().unwrap().get(&task_name) {
                        Some(t) => t.write().unwrap().runtime_info = runtime_info,
                        None => {
                            // Task was unscheduled but I just started it... TODO: do something
                            // here?
                            continue;
                        }
                    }
                } else {
                    match self.0.tasks.read().unwrap().get(&task_name) {
                        Some(t) => t.write().unwrap().runtime_info.state = TaskState::Failed,
                        None => continue,
                    }
                }
            }
        }
    }

    fn execute(&self, diff: &DiffResponse) -> Result<Vec<Task>, MetalMonitorError> {
        let mut results = Vec::new();
        for added in &diff.added.tasks {
            // Update or create the task lock entry
            {
                let mut _taskmap = self.0.tasks.write().unwrap();
                let mut task = _taskmap
                    .entry(added.name.to_owned())
                    .or_insert_with(|| RwLock::new(added.clone()))
                    .write()
                    .unwrap();

                let mut added = added.clone();
                added.runtime_info = task.runtime_info.clone();
                *task = added
            }

            // Re-acquire the taskmap as a readlock, attempt to stop/start the task
            {
                let _taskmap = self.0.tasks.read().unwrap();
                let task_lock = _taskmap
                    .get(&added.name)
                    .ok_or(MetalMonitorError::ConcurrencyError)?;

                let mut task = task_lock.write().unwrap();
                let runtime_info = self.stop_task(&task)?;
                task.runtime_info = runtime_info;
                let runtime_info = self.start_task(&task)?;
                task.runtime_info = runtime_info;
                results.push(task.clone());
            }
        }

        for removed in &diff.removed.tasks {
            let mut _taskmap = self.0.tasks.read().unwrap();
            let mut task = match _taskmap.get(&removed.name) {
                Some(t) => t.write().unwrap(),
                // No need to stop it if it doesn't exist
                None => continue,
            };

            let runtime_info = self.stop_task(&task)?;
            task.runtime_info = runtime_info.clone();
            task.runtime_info.state = TaskState::Stopped;
            results.push(task.clone());
        }

        // Report started/changed tasks
        let mut to_clean_up = Vec::new();
        if !results.is_empty() {
            if let Some(c) = &*self.0.coordinator.lock().unwrap() {
                to_clean_up = c.report_tasks(
                    results
                        .iter()
                        .map(|t| (t.name.to_owned(), t.runtime_info.clone()))
                        .collect(),
                );
            }
        }

        // Clean up removed tasks
        if !to_clean_up.is_empty() {
            let mut _tasks = self.0.tasks.write().unwrap();
            for task_name in &to_clean_up {
                _tasks.remove(task_name);
            }
        }

        Ok(results)
    }
}

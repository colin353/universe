use metal_grpc_rust::Task;
use protobuf::Message;

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub enum MetalStateError {
    FilesystemError(std::io::Error),
    ProtobufError(protobuf::ProtobufError),
}

pub trait MetalStateManager: Send + Sync {
    fn initialize(&self) -> Result<(), MetalStateError> {
        Ok(())
    }
    fn set_task(&self, task: &Task) -> Result<(), MetalStateError>;
    fn get_task(&self, name: &str) -> Result<Option<Task>, MetalStateError>;
    fn all_tasks(&self) -> Result<Vec<Task>, MetalStateError>;
}

pub struct FilesystemState {
    root: Mutex<std::path::PathBuf>,
}

fn path_from_resource_name(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let mut p = root.to_owned();
    p.push("resources");
    for component in name.split(".") {
        p.push(component)
    }
    p
}

impl FilesystemState {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self {
            root: Mutex::new(root),
        }
    }

    fn initialize(&self) -> Result<(), MetalStateError> {
        let root = self.root.lock().unwrap();
        let res = root.join("resources");
        if !res.exists() {
            match std::fs::create_dir_all(&*res) {
                Ok(_) => return Ok(()),
                Err(e) => return Err(MetalStateError::FilesystemError(e)),
            };
        }
        Ok(())
    }

    fn read_task(full_path: &std::path::Path) -> Result<Option<Task>, MetalStateError> {
        let mut f = match std::fs::File::open(full_path) {
            Ok(f) => f,
            Err(e) => {
                // File not found is not an error
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Ok(None);
                }
                return Err(MetalStateError::FilesystemError(e));
            }
        };
        match protobuf::parse_from_reader(&mut f) {
            Ok(t) => Ok(Some(t)),
            Err(e) => return Err(MetalStateError::ProtobufError(e)),
        }
    }

    fn extract_tasks(dir: &std::path::Path, tasks: &mut Vec<Task>) -> Result<(), MetalStateError> {
        let iter = match std::fs::read_dir(dir) {
            Ok(i) => i,
            Err(e) => return Err(MetalStateError::FilesystemError(e)),
        };

        for entry in iter {
            let path = match entry {
                Ok(e) => e.path(),
                Err(e) => return Err(MetalStateError::FilesystemError(e)),
            };
            if path.is_dir() {
                Self::extract_tasks(&path, tasks)?;
                continue;
            }
            if path.extension() == Some(std::ffi::OsStr::new("task")) {
                if let Some(t) = Self::read_task(&path)? {
                    tasks.push(t);
                }
            }
        }

        Ok(())
    }
}

impl MetalStateManager for FilesystemState {
    fn initialize(&self) -> Result<(), MetalStateError> {
        self.initialize()
    }

    fn set_task(&self, task: &Task) -> Result<(), MetalStateError> {
        let root = self.root.lock().unwrap();
        let mut filename = path_from_resource_name(&root, task.get_name());
        filename.set_extension("task");
        let mut f = match std::fs::File::create(filename) {
            Ok(f) => f,
            Err(e) => return Err(MetalStateError::FilesystemError(e)),
        };
        match task.write_to_writer(&mut f) {
            Ok(_) => Ok(()),
            Err(e) => return Err(MetalStateError::ProtobufError(e)),
        }
    }

    fn get_task(&self, name: &str) -> Result<Option<Task>, MetalStateError> {
        let root = self.root.lock().unwrap();
        let mut filename = path_from_resource_name(&root, name);
        filename.set_extension("task");
        Self::read_task(&filename)
    }

    fn all_tasks(&self) -> Result<Vec<Task>, MetalStateError> {
        let root = self.root.lock().unwrap();
        let res = root.join("resources");
        let mut tasks = Vec::new();
        Self::extract_tasks(&res, &mut tasks)?;
        Ok(tasks)
    }
}

pub struct FakeState {
    tasks: Mutex<HashMap<String, Task>>,
}

impl FakeState {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
        }
    }
}

impl MetalStateManager for FakeState {
    fn set_task(&self, task: &Task) -> Result<(), MetalStateError> {
        self.tasks
            .lock()
            .unwrap()
            .insert(task.get_name().to_string(), task.clone());
        Ok(())
    }

    fn get_task(&self, name: &str) -> Result<Option<Task>, MetalStateError> {
        Ok(self.tasks.lock().unwrap().get(name).map(|t| t.clone()))
    }

    fn all_tasks(&self) -> Result<Vec<Task>, MetalStateError> {
        Ok(self
            .tasks
            .lock()
            .unwrap()
            .iter()
            .map(|(_, t)| t.clone())
            .collect())
    }
}

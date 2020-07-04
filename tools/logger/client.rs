use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use grpc::ClientStubExt;

pub use log_types_proto_rust::*;
use logger_grpc_rust::*;

pub use logger_grpc_rust::Log;

#[derive(Clone)]
pub struct LoggerClient {
    client: Option<Arc<LoggerServiceClient>>,
    logcache: Arc<RwLock<HashMap<Log, Mutex<Vec<EventMessage>>>>>,
}

pub fn get_timestamp() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as u64
}

pub fn get_timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

pub fn get_date_dir(timestamp: u64) -> String {
    let mut epoch = time::empty_tm();
    epoch.tm_mday = 1;
    epoch.tm_year = 70;
    let epoch = epoch + time::Duration::seconds(timestamp as i64);

    format!(
        "{}/{:02}/{:02}",
        epoch.tm_year + 1900,
        epoch.tm_mon + 1,
        epoch.tm_mday
    )
}

pub fn get_log_dir(root_dir: &str, log: Log, timestamp: u64) -> String {
    format!(
        "{}/{}/{}",
        root_dir,
        get_log_name(log),
        get_date_dir(timestamp)
    )
}

pub fn get_log_name(log: Log) -> &'static str {
    match log {
        Log::UNKNOWN => "unknown",
        Log::LARGETABLE_READS => "LargetableReadLog",
    }
}

pub fn get_logs_with_root_dir(
    root_dir: &str,
    log: Log,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Vec<EventMessage> {
    if end_timestamp < start_timestamp {
        return Vec::new();
    }

    let mut files_to_read = Vec::new();
    let mut timestamp = start_timestamp;
    let final_log_dir = get_log_dir(root_dir, log, end_timestamp);

    let mut attempts = 0;
    loop {
        attempts += 1;
        let dir = get_log_dir(root_dir, log, timestamp);
        println!("check dir: {}", dir);

        match gfile::GFile::read_dir(&dir) {
            Ok(mut filenames) => files_to_read.append(&mut filenames),
            // Just ignore errors, since it means the dir doesn't exist,
            // which means no logs for that date
            Err(_) => (),
        };

        if dir == final_log_dir {
            break;
        }

        if attempts > 365 {
            panic!("too many attempts!");
        }

        // Advance the timestamp by 24h
        timestamp += 3600 * 24;
    }

    let mut output = Vec::new();
    for file in files_to_read {
        let f = gfile::GFile::open(file).unwrap();
        let mut buf = std::io::BufReader::new(f);
        let reader = recordio::RecordIOReader::<EventMessage, _>::new(&mut buf);
        for record in reader {
            let timestamp_seconds = record.get_event_id().get_timestamp() / 1_000_000;
            if timestamp_seconds < start_timestamp || timestamp_seconds > end_timestamp {
                continue;
            }

            output.push(record);
        }
    }

    output.sort_unstable_by_key(|e| e.get_event_id().get_timestamp());
    output
}

impl LoggerClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Some(Arc::new(
                LoggerServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            )),
            logcache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn new_stdout() -> Self {
        Self {
            client: None,
            logcache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn log<T: protobuf::Message>(&self, log: Log, input: &T) {
        if self.client.is_none() {
            println!("{:?}\t{:?}", log, input);
            return;
        }

        let mut m = EventMessage::new();
        input.write_to_writer(m.mut_msg()).unwrap();
        {
            let eid = m.mut_event_id();
            eid.set_timestamp(get_timestamp_usec());
            eid.set_pid(std::process::id());
        }

        match self.logcache.read().unwrap().get(&log) {
            Some(l) => {
                l.lock().unwrap().push(m);
                return;
            }
            None => {}
        };
        let logs = vec![m];
        self.logcache.write().unwrap().insert(log, Mutex::new(logs));
    }

    pub fn start_logging(&self) {
        if self.client.is_none() {
            return;
        }

        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));

            // Iterate over available keys
            let keys: Vec<_> = self.logcache.read().unwrap().keys().map(|k| *k).collect();
            for key in keys {
                let data = match self.logcache.read().unwrap().get(&key) {
                    Some(logs) => {
                        let mut data = logs.lock().unwrap();
                        Some(std::mem::replace(&mut *data, Vec::new()))
                    }
                    None => None,
                };

                match data {
                    Some(logs) => {
                        let mut req = LogRequest::new();
                        req.set_log(key);
                        *req.mut_messages() = protobuf::RepeatedField::from_vec(logs);
                        match self
                            .client
                            .as_ref()
                            .unwrap()
                            .log(Default::default(), req)
                            .wait()
                        {
                            Ok(_) => (),
                            Err(_) => eprintln!("Failed to log some messages! Logs lost!"),
                        };
                    }
                    None => (),
                }
            }
        }
    }
}

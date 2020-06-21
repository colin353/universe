use logger_grpc_rust::*;
use rand::Rng;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone)]
pub struct LoggerServiceHandler {
    data_dir: String,
    writers: Arc<RwLock<HashMap<Log, Mutex<recordio::RecordIOWriterOwned<EventMessage>>>>>,
}

pub fn get_timestamp() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as u64
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

pub fn random_filename() -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .collect::<String>()
}

impl LoggerServiceHandler {
    pub fn new(data_dir: String) -> Self {
        Self {
            writers: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    // Create a writer for this log if it doesn't already exist
    pub fn make_writer(&self, log: Log) {
        let filename = format!(
            "{}/{}.recordio",
            get_log_dir(&self.data_dir, log, get_timestamp()),
            random_filename()
        );
        let f = gfile::GFile::create(filename).unwrap();
        let buf = std::io::BufWriter::new(f);
        let w = recordio::RecordIOWriterOwned::new(Box::new(buf));
        self.writers.write().unwrap().insert(log, Mutex::new(w));
    }

    pub fn log(&self, mut req: LogRequest) -> LogResponse {
        for _ in 0..2 {
            {
                let _w = self.writers.read().unwrap();
                match _w.get(&req.get_log()) {
                    Some(logger) => {
                        for event in req.take_messages().into_iter() {
                            logger.lock().unwrap().write(&event);
                        }
                        return LogResponse::new();
                    }
                    None => (),
                }
            };

            // If we end up here, it means we don't yet have a
            // writer constructed for this log, so let's construct one
            self.make_writer(req.get_log());
        }

        panic!("failed to make writer!");
    }

    pub fn get_logs(&self, req: GetLogsRequest) -> GetLogsResponse {
        GetLogsResponse::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directories() {
        assert_eq!(
            get_log_dir("/data", Log::UNKNOWN, 0),
            "/data/unknown/1970/01/01"
        );
        assert_eq!(
            get_log_dir("/data", Log::LARGETABLE_READS, 1592761499),
            "/data/LargetableReadLog/2020/06/21"
        );
    }

    //#[test]
    fn test_logger() {
        let l = LoggerServiceHandler::new("/tmp/data".to_string());
        let mut req = LogRequest::new();
        req.set_log(Log::LARGETABLE_READS);

        let mut em = EventMessage::new();
        em.mut_event_id().set_timestamp(1234);
        em.mut_event_id().set_ip_address(vec![1, 2, 3, 4]);

        req.mut_messages().push(em.clone());
        req.mut_messages().push(em);
        l.log(req);
    }
}

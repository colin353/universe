use logger_grpc_rust::*;
use rand::Rng;

use itertools::{MinHeap, KV};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use logger_client::{get_date_dir, get_log_dir, get_logs_with_root_dir, get_timestamp};

#[derive(Clone)]
pub struct LoggerServiceHandler {
    data_dir: String,
    cns_data_dir: String,
    writers: Arc<RwLock<HashMap<Log, Mutex<(recordio::RecordIOWriterOwned<EventMessage>, u64)>>>>,
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
            cns_data_dir: String::from("/cns/colossus/logs"),
        }
    }

    // Create a writer for this log if it doesn't already exist
    pub fn make_writer(&self, log: Log) {
        let t = get_timestamp();
        let filename = format!(
            "{}/{}.recordio",
            get_log_dir(&self.data_dir, log, t),
            random_filename()
        );
        let f = gfile::GFile::create(filename).unwrap();
        let w = recordio::RecordIOWriterOwned::new(Box::new(f));
        self.writers
            .write()
            .unwrap()
            .insert(log, Mutex::new((w, t)));
    }

    pub fn log(&self, mut req: LogRequest) -> LogResponse {
        for _ in 0..2 {
            {
                let _w = self.writers.read().unwrap();
                match _w.get(&req.get_log()) {
                    Some(logger) => {
                        for event in req.take_messages().into_iter() {
                            logger.lock().unwrap().0.write(&event);
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
        let logs = get_logs_with_root_dir(
            &self.data_dir,
            req.get_log(),
            req.get_start_time(),
            req.get_end_time(),
        );
        let mut resp = GetLogsResponse::new();
        resp.set_messages(protobuf::RepeatedField::from_vec(logs));
        resp
    }

    pub fn reorganize(&self) {
        // Check whether any of the logs have expired
        let mut expired_logs = Vec::new();
        let t = get_timestamp();
        let date_dir = get_date_dir(t);
        {
            let map = self.writers.read().unwrap();
            for (l, m) in map.iter() {
                let creation = m.lock().unwrap().1;
                if t - creation > 3600 || date_dir != get_date_dir(creation) {
                    expired_logs.push(l.clone());
                }
            }
        }

        for log in expired_logs {
            self.make_writer(log);
        }

        let logs = match gfile::GFile::read_dir(&self.data_dir) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("Couldn't read data directory!");
                return;
            }
        };

        let mut logs_to_move = HashMap::new();
        for log in logs {
            let mut log_name_split = log[self.data_dir.len() + 1..].split("/");
            let log_name = match log_name_split.next() {
                Some(x) => x,
                None => continue,
            };

            let year = match log_name_split.next() {
                Some(x) => x,
                None => continue,
            };
            let month = match log_name_split.next() {
                Some(x) => x,
                None => continue,
            };
            let day = match log_name_split.next() {
                Some(x) => x,
                None => continue,
            };

            if log.starts_with(&format!("{}/{}/{}", self.data_dir, log_name, date_dir)) {
                continue;
            }

            let l = logs_to_move
                .entry((log_name.to_string(), format!("{}/{}/{}", year, month, day)))
                .or_insert(Vec::new());
            l.push(log.clone());
        }

        for ((log_name, date_dir), logs) in logs_to_move {
            self.aggregate_logs(&log_name, &date_dir, logs);
        }
    }

    pub fn aggregate_logs(&self, log_name: &str, date_dir: &str, files: Vec<String>) {
        let mut readers: Vec<_> = files
            .iter()
            .map(|f| {
                let file = gfile::GFile::open(f).unwrap();
                let f = std::io::BufReader::new(file);
                recordio::RecordIOReaderOwned::<EventMessage>::new(Box::new(f))
            })
            .collect();

        let mut heap = MinHeap::new();
        for (idx, reader) in readers.iter_mut().enumerate() {
            for _ in 0..100 {
                let record = match reader.read() {
                    Some(r) => r,
                    None => break,
                };
                heap.push(KV::new(
                    record.get_event_id().get_timestamp(),
                    (record, idx),
                ));
            }
        }

        let output = gfile::GFile::create(format!(
            "{}/{}/{}/logs.recordio",
            self.cns_data_dir, log_name, date_dir
        ))
        .unwrap();
        let mut writer = recordio::RecordIOWriterOwned::new(Box::new(output));
        while let Some(KV(_, (record, idx))) = heap.pop() {
            writer.write(&record);

            let record = match readers[idx].read() {
                Some(r) => r,
                None => continue,
            };
            heap.push(KV::new(
                record.get_event_id().get_timestamp(),
                (record, idx),
            ));
        }

        // Delete the log files
        let dir_to_delete = format!("{}/{}/{}", self.data_dir, log_name, date_dir);
        match std::fs::remove_dir_all(&dir_to_delete) {
            Ok(_) => (),
            Err(e) => eprintln!("failed to delete dir: `{}`: {:?}", dir_to_delete, e),
        };
    }
}

impl logger_grpc_rust::LoggerService for LoggerServiceHandler {
    fn log(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<LogRequest>,
        resp: grpc::ServerResponseUnarySink<LogResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.log(req.message))
    }

    fn get_logs(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<GetLogsRequest>,
        resp: grpc::ServerResponseUnarySink<GetLogsResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.get_logs(req.message))
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
        em.mut_event_id().set_timestamp(get_timestamp() * 1_000_000);
        em.mut_event_id().set_ip_address(vec![1, 2, 3, 4]);

        req.mut_messages().push(em.clone());
        req.mut_messages().push(em);
        l.log(req);
    }

    //#[test]
    fn test_get_logs() {
        let logs = get_logs_with_root_dir(
            "/tmp/data",
            Log::LARGETABLE_READS,
            get_timestamp() - 172800,
            get_timestamp(),
        );
        assert_eq!(logs, Vec::new());
        assert_eq!(logs.len(), 1);
    }
}

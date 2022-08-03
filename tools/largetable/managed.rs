use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

use std::os::unix::fs::MetadataExt;

pub fn timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

pub struct ManagedLargeTable {
    pub table: RwLock<largetable::LargeTable<'static, std::io::BufWriter<std::fs::File>>>,
    memory_limit: u64,
    throttler: Throttler,
    journals: Mutex<Vec<std::path::PathBuf>>,
    data_path: std::path::PathBuf,
}

impl ManagedLargeTable {
    pub fn new(data_path: std::path::PathBuf) -> std::io::Result<Self> {
        let mut table = largetable::LargeTable::new();
        table.add_mtable();

        let mut journals = Vec::new();

        let dtable_extension = std::ffi::OsStr::new("dtable");
        let journal_extension = std::ffi::OsStr::new("journal");

        for entry in std::fs::read_dir(&data_path)? {
            let path = entry?.path();
            if let Some(ext) = path.extension() {
                if ext == dtable_extension {
                    let f = std::fs::File::open(&path)?;
                    let dt = largetable::DTable::from_file(f).expect("corrupt dtable?");
                    table.add_dtable(dt);
                } else if ext == journal_extension {
                    // Delete any zero-size journals, which can accumulate from process restarts
                    let f = std::fs::File::open(&path)?;
                    if f.metadata()?.len() == 0 {
                        std::fs::remove_file(&path)?;
                        continue;
                    }

                    let r = std::io::BufReader::new(f);
                    table.load_from_journal(r)?;
                    journals.push(path);
                }
            }
        }

        let journal_path = data_path.join(format!("{}.journal", timestamp_usec()));
        let f = std::fs::File::create(&journal_path)?;
        table.add_journal(std::io::BufWriter::new(f));
        journals.insert(0, journal_path);

        Ok(Self {
            table: RwLock::new(table),
            memory_limit: 512_000_000,
            throttler: Throttler::new(),
            journals: Mutex::new(journals),
            data_path,
        })
    }

    pub fn read<T: bus::DeserializeOwned>(
        &self,
        row: &str,
        column: &str,
        timestamp: u64,
    ) -> Option<std::io::Result<T>> {
        let timestamp = match timestamp {
            0 => timestamp_usec(),
            x => x,
        };

        self.table
            .read()
            .expect("failed to read lock largetable")
            .read(row, column, timestamp)
    }

    pub fn delete(
        &self,
        row: String,
        column: String,
        timestamp: u64,
    ) -> std::io::Result<service::DeleteResponse> {
        let timestamp = match timestamp {
            0 => timestamp_usec(),
            x => x,
        };

        self.table
            .read()
            .expect("failed to read lock largetable")
            .delete(row.to_owned(), column, timestamp)?;

        Ok(service::DeleteResponse { timestamp })
    }

    pub fn write<T: bus::Serialize>(
        &self,
        row: String,
        column: String,
        timestamp: u64,
        data: T,
    ) -> std::io::Result<service::WriteResponse> {
        let timestamp = match timestamp {
            0 => timestamp_usec(),
            x => x,
        };

        self.table
            .read()
            .expect("failed to read lock largetable")
            .write(row.to_owned(), column, timestamp, data)?;

        Ok(service::WriteResponse { timestamp })
    }

    pub fn read_range(
        &self,
        filter: largetable::Filter,
        timestamp: u64,
        limit: usize,
    ) -> std::io::Result<service::ReadRangeResponse> {
        let timestamp = match timestamp {
            0 => timestamp_usec(),
            x => x,
        };

        let results: Vec<(String, bus::PackedIn<u8>)> = self
            .table
            .read()
            .expect("failed to read lock largetable")
            .read_range(filter, timestamp, limit)?;

        Ok(service::ReadRangeResponse {
            records: results
                .into_iter()
                .map(|(key, data)| service::Record { key, data: data.0 })
                .collect(),
            timestamp,
        })
    }

    pub fn monitor_memory(&self) {
        let mut last_check = std::time::Instant::now();
        let mut last_memory = {
            let _read = self.table.read().expect("failed to read lock largetable");
            _read.memory_usage()
        };

        let mut throttling_enabled = false;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));

            let memory_usage = {
                let _read = self.table.read().expect("failed to read lock largetable");
                _read.memory_usage()
            };

            let bytes = if memory_usage > last_memory {
                memory_usage - last_memory
            } else {
                0
            };

            let throttling = self.throttler.update(bytes as usize, last_check.elapsed());

            if throttling {
                println!(
                    "{} bytes/s",
                    bytes as f64 / last_check.elapsed().as_secs_f64()
                );
            }

            if throttling != throttling_enabled {
                println!(
                    "throttling {}",
                    if throttling { "enabled" } else { "disabled" }
                );
            }
            throttling_enabled = throttling;

            last_check = std::time::Instant::now();
            last_memory = memory_usage;

            if memory_usage < self.memory_limit {
                continue;
            }

            println!("memory limit exceeded, persisting to disk...");
            last_memory = 0;

            // Insert a new mtable at the zero position, so that all writes are redirected to that
            let journal_path = self.data_path.join(format!("{}.journal", timestamp_usec()));
            let f = std::fs::File::create(&journal_path).expect("failed to create journal!");
            {
                let mut _write = self.table.write().expect("failed to write lock largetable");

                _write.add_journal(std::io::BufWriter::new(f));
                _write.add_mtable();
            }

            // Write the mtable to disk
            let dtable_path = self.data_path.join(format!("{}.dtable", timestamp_usec()));
            let f = std::fs::File::create(&dtable_path).expect("failed to create dtable!");
            let persist_start = std::time::Instant::now();
            {
                let _read = self.table.read().expect("failed to read lock largetable");
                _read.mtables[1]
                    .read()
                    .expect("failed to read lock mtable")
                    .write_to_dtable(std::io::BufWriter::new(f))
                    .expect("failed to persist mtable to disk!");
            }
            let metadata =
                std::fs::metadata(&dtable_path).expect("failed to read dtable metadata!");
            println!(
                "wrote {} bytes in {:?} ({} bytes/second)",
                metadata.size(),
                persist_start.elapsed(),
                metadata.size() as f64 / persist_start.elapsed().as_secs_f64(),
            );

            // Load the new dtable from disk
            let f = std::fs::File::open(&dtable_path).expect("failed to create dtable!");
            let dtable = largetable::DTable::from_file(f).expect("failed to load dtable");
            {
                let mut _write = self.table.write().expect("failed to write lock largetable");
                _write.add_dtable(dtable);

                // Discard the mtable
                _write.drop_read_only_mtable();
            }

            // Delete the journals that were used to construct the loaded DTable
            let mut _w = self.journals.lock().expect("failed to lock journals");
            for path in _w.iter() {
                if let Err(e) = std::fs::remove_file(&path) {
                    eprintln!("failed to delete journal: {:?}", e);
                }
            }
            _w.clear();
            _w.push(journal_path);
        }
    }
}

impl service::LargeTableServiceHandler for ManagedLargeTable {
    fn read(&self, req: service::ReadRequest) -> Result<service::ReadResponse, bus::BusRpcError> {
        let buf: Option<std::io::Result<bus::PackedIn<u8>>> =
            self.read(&req.row, &req.column, req.timestamp);

        match buf {
            Some(Ok(data)) => Ok(service::ReadResponse {
                found: true,
                data: data.0,
                timestamp: req.timestamp,
            }),
            Some(Err(e)) => return Err(bus::BusRpcError::InvalidData(e)),
            None => Ok(service::ReadResponse {
                found: false,
                data: Vec::new(),
                timestamp: req.timestamp,
            }),
        }
    }

    fn write(
        &self,
        req: service::WriteRequest,
    ) -> Result<service::WriteResponse, bus::BusRpcError> {
        self.throttler
            .maybe_throttle(req.data.len())
            .map_err(|_| bus::BusRpcError::BackOff)?;
        self.write(
            req.row,
            req.column,
            req.timestamp,
            bus::PackedOut(&req.data),
        )
        .map_err(|e| bus::BusRpcError::InvalidData(e))
    }

    fn read_range(
        &self,
        req: service::ReadRangeRequest,
    ) -> Result<service::ReadRangeResponse, bus::BusRpcError> {
        let f = largetable::Filter {
            row: &req.row,
            spec: &req.filter.spec,
            min: &req.filter.min,
            max: &req.filter.max,
        };

        self.read_range(f, req.timestamp, std::cmp::min(req.limit as usize, 1024))
            .map_err(|e| bus::BusRpcError::InvalidData(e))
    }

    fn write_bulk(
        &self,
        req: service::WriteBulkRequest,
    ) -> Result<service::WriteBulkResponse, bus::BusRpcError> {
        for w in req.writes {
            self.write(w.row, w.column, w.timestamp, bus::PackedOut(&w.data))
                .map_err(|e| bus::BusRpcError::InvalidData(e))?;
        }
        Ok(service::WriteBulkResponse::new())
    }

    fn delete(
        &self,
        req: service::DeleteRequest,
    ) -> Result<service::DeleteResponse, bus::BusRpcError> {
        Ok(self
            .delete(req.row, req.column, req.timestamp)
            .map_err(|e| bus::BusRpcError::InternalError(format!("{:?}", e)))?)
    }
}

pub struct Throttler {
    throttle: AtomicUsize,
    accumulator: AtomicIsize,
    proportional: f64,
    integral: f64,
    target: usize,
}

impl Throttler {
    fn new() -> Self {
        Throttler {
            throttle: AtomicUsize::new(0),
            proportional: 1.0,
            integral: 0.2,
            accumulator: AtomicIsize::new(0),
            target: 100_000_000,
        }
    }

    pub fn update(&self, bytes: usize, time: std::time::Duration) -> bool {
        let throughput = bytes as f64 / time.as_secs_f64();

        if throughput < (self.target as f64) / 2.0 {
            self.accumulator.store(0, Ordering::SeqCst);
            self.throttle.store(0, Ordering::SeqCst);
            return false;
        }

        let target_throughput = self.target as f64;
        let error = (throughput as isize) - target_throughput as isize;
        let value = self.accumulator.fetch_add(error, Ordering::SeqCst);
        let accumulator = error + value;

        // The throttle is measured in units of throughput, bytes per second.
        let throttle: f64 = error as f64 * self.proportional + accumulator as f64 * self.integral;
        if throttle < 0.0 {
            self.accumulator.store(0, Ordering::SeqCst);
            self.throttle.store(0, Ordering::SeqCst);
            return false;
        }

        // Desired slowdown per byte = throttle / throughput * (1/throughput)
        let setting = ((throttle / throughput) * (1_000_000_000.0 / bytes as f64)) as usize;

        self.throttle.store(setting, Ordering::SeqCst);
        return true;
    }

    pub fn maybe_throttle(&self, bytes: usize) -> std::io::Result<()> {
        let ns = self.throttle.load(Ordering::SeqCst) * bytes;
        if ns == 0 {
            return Ok(());
        }

        if ns < 200_000_000 {
            std::thread::sleep(std::time::Duration::from_nanos(ns as u64));
            return Ok(());
        }

        Err(std::io::Error::from(std::io::ErrorKind::TimedOut))
    }
}

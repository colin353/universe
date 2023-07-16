use service::*;

use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use std::os::unix::fs::MetadataExt;

pub fn timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

pub struct LargeTableHandler {
    table: RwLock<largetable::LargeTable<'static, std::io::BufWriter<std::fs::File>>>,
    memory_limit: u64,
    throttler: Throttler,
    journals: Mutex<Vec<std::path::PathBuf>>,
}

impl LargeTableHandler {
    pub fn new(
        table: largetable::LargeTable<'static, std::io::BufWriter<std::fs::File>>,
        journals: Vec<std::path::PathBuf>,
    ) -> Self {
        Self {
            table: RwLock::new(table),
            memory_limit: 512_000_000,
            throttler: Throttler::new(),
            journals: Mutex::new(journals),
        }
    }
}

impl LargeTableServiceHandler for LargeTableHandler {
    fn read(&self, req: ReadRequest) -> Result<ReadResponse, bus::BusRpcError> {
        let timestamp = match req.timestamp {
            0 => timestamp_usec(),
            x => x,
        };

        let buf: Option<std::io::Result<bus::PackedIn<u8>>> = self
            .table
            .read()
            .expect("failed to read lock largetable")
            .read(&req.row, &req.column, timestamp);

        match buf {
            Some(Ok(data)) => Ok(ReadResponse {
                found: true,
                data: data.0,
                timestamp,
            }),
            Some(Err(e)) => {
                eprintln!("{:?}", e);
                Err(bus::BusRpcError::InternalError(String::from(
                    "failed to read from largetable",
                )))
            }
            None => Ok(ReadResponse {
                found: false,
                data: Vec::new(),
                timestamp,
            }),
        }
    }

    fn write(&self, req: WriteRequest) -> Result<WriteResponse, bus::BusRpcError> {
        self.throttler.maybe_throttle(req.data.len())?;

        let timestamp = match req.timestamp {
            0 => timestamp_usec(),
            x => x,
        };

        let data = bus::PackedOut(&req.data);
        self.table
            .read()
            .expect("failed to read lock largetable")
            .write(req.row, req.column, timestamp, data)
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError(String::from("failed to read from largetable"))
            })?;

        Ok(WriteResponse { timestamp })
    }

    fn read_range(&self, req: ReadRangeRequest) -> Result<ReadRangeResponse, bus::BusRpcError> {
        let timestamp = match req.timestamp {
            0 => timestamp_usec(),
            x => x,
        };

        let f = largetable::Filter {
            row: &req.row,
            spec: &req.filter.spec,
            min: &req.filter.min,
            max: &req.filter.max,
        };

        let results: Vec<(String, bus::PackedIn<u8>)> = self
            .table
            .read()
            .expect("failed to read lock largetable")
            .read_range(f, timestamp, std::cmp::min(req.limit as usize, 1024))
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError(String::from(
                    "failed to read_range from largetable",
                ))
            })?;

        let resp = Ok(ReadRangeResponse {
            records: results
                .into_iter()
                .map(|(key, data)| Record { key, data: data.0 })
                .collect(),
            timestamp,
        });

        resp
    }

    fn write_bulk(&self, req: WriteBulkRequest) -> Result<WriteBulkResponse, bus::BusRpcError> {
        for w in req.writes {
            self.write(w)?;
        }
        Ok(WriteBulkResponse::new())
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

    pub fn maybe_throttle(&self, bytes: usize) -> Result<(), bus::BusRpcError> {
        let ns = self.throttle.load(Ordering::SeqCst) * bytes;
        if ns == 0 {
            return Ok(());
        }

        if ns < 200_000_000 {
            std::thread::sleep(std::time::Duration::from_nanos(ns as u64));
            return Ok(());
        }

        Err(bus::BusRpcError::BackOff)
    }
}

pub fn monitor_memory(data_path: std::path::PathBuf, handler: Arc<LargeTableHandler>) {
    let mut last_check = std::time::Instant::now();
    let mut last_memory = {
        let _read = handler
            .table
            .read()
            .expect("failed to read lock largetable");
        _read.memory_usage()
    };

    let mut throttling_enabled = false;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        let memory_usage = {
            let _read = handler
                .table
                .read()
                .expect("failed to read lock largetable");
            _read.memory_usage()
        };

        let bytes = if memory_usage > last_memory {
            memory_usage - last_memory
        } else {
            0
        };

        let throttling = handler
            .throttler
            .update(bytes as usize, last_check.elapsed());

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

        if memory_usage < handler.memory_limit {
            continue;
        }

        println!("memory limit exceeded, persisting to disk...");
        last_memory = 0;

        // Insert a new mtable at the zero position, so that all writes are redirected to that
        let journal_path = data_path.join(format!("{}.journal", timestamp_usec()));
        let f = std::fs::File::create(&journal_path).expect("failed to create journal!");
        {
            let mut _write = handler
                .table
                .write()
                .expect("failed to write lock largetable");

            _write.add_journal(std::io::BufWriter::new(f));
            _write.add_mtable();
        }

        // Write the mtable to disk
        let dtable_path = data_path.join(format!("{}.dtable", timestamp_usec()));
        let f = std::fs::File::create(&dtable_path).expect("failed to create dtable!");
        let persist_start = std::time::Instant::now();
        {
            let _read = handler
                .table
                .read()
                .expect("failed to read lock largetable");
            _read.mtables[1]
                .read()
                .expect("failed to read lock mtable")
                .write_to_dtable(std::io::BufWriter::new(f))
                .expect("failed to persist mtable to disk!");
        }
        let metadata = std::fs::metadata(&dtable_path).expect("failed to read dtable metadata!");
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
            let mut _write = handler
                .table
                .write()
                .expect("failed to write lock largetable");
            _write.add_dtable(dtable);

            // Discard the mtable
            _write.drop_read_only_mtable();
        }

        // Delete the journals that were used to construct the loaded DTable
        let mut _w = handler.journals.lock().expect("failed to lock journals");
        for path in _w.iter() {
            if let Err(e) = std::fs::remove_file(&path) {
                eprintln!("failed to delete journal: {:?}", e);
            }
        }
        _w.clear();
        _w.push(journal_path);
    }
}

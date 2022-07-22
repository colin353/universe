use service::*;

use std::sync::{Arc, RwLock};

pub fn timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

pub struct LargeTableHandler {
    table: RwLock<largetable::LargeTable<'static, std::io::BufWriter<std::fs::File>>>,
    memory_limit: u64,
}

impl LargeTableHandler {
    pub fn new(table: largetable::LargeTable<'static, std::io::BufWriter<std::fs::File>>) -> Self {
        Self {
            table: RwLock::new(table),
            memory_limit: 100_000_000,
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
}

pub fn monitor_memory(data_path: std::path::PathBuf, handler: Arc<LargeTableHandler>) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        {
            let _read = handler
                .table
                .read()
                .expect("failed to read lock largetable");
            if _read.memory_usage() < handler.memory_limit {
                continue;
            }
        }

        println!("memory limit exceeded, persisting to disk...");

        // Insert a new mtable at the zero position, so that all writes are redirected to that
        {
            let mut _write = handler
                .table
                .write()
                .expect("failed to write lock largetable");
            _write.add_mtable();
        }

        // Write the mtable to disk
        let dtable_path = data_path.join(format!("{}.dtable", timestamp_usec()));
        let f = std::fs::File::create(&dtable_path).expect("failed to create dtable!");
        {
            let _read = handler
                .table
                .read()
                .expect("failed to read lock largetable");
            _read.mtables[1]
                .read()
                .expect("failed to read lock mtable")
                .write_to_dtable(std::io::BufWriter::new(&f))
                .expect("failed to persist mtable to disk!");
        }

        // Load the new dtable from disk
        let dtable = largetable::DTable::from_file(f).expect("failed to load dtable");
        let mut _write = handler
            .table
            .write()
            .expect("failed to write lock largetable");
        _write.add_dtable(dtable);

        // Discard the mtable
        _write.drop_read_only_mtable();

        println!("wrote and loaded {:?}", dtable_path);
    }
}

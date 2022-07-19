use service::*;

use std::sync::RwLock;

pub fn timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

pub struct LargeTableHandler {
    table: RwLock<largetable::LargeTable<'static, std::io::BufWriter<std::fs::File>>>,
}

impl LargeTableHandler {
    pub fn new(table: largetable::LargeTable<'static, std::io::BufWriter<std::fs::File>>) -> Self {
        Self {
            table: RwLock::new(table),
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

/*
 * serviceimpl.rs
 *
 * This file implements the largetable service.
 */

use largetable;
use largetable_grpc_rust;

use glob;
use protobuf;
use time;

use std::fs;
use std::io::Write;
use std::path;
use std::sync::Arc;
use std::sync::RwLock;

const DTABLE_EXT: &'static str = "sstable";
const JOURNAL_EXT: &'static str = "recordio";

#[derive(Clone)]
pub struct LargeTableServiceHandler {
    largetable: Arc<RwLock<largetable::LargeTable>>,
    memory_limit: u64,
    data_directory: String,
    next_file_number: u64,
}

impl LargeTableServiceHandler {
    pub fn new(memory_limit: u64, data_directory: String) -> LargeTableServiceHandler {
        LargeTableServiceHandler {
            largetable: Arc::new(RwLock::new(largetable::LargeTable::new())),
            memory_limit,
            data_directory,
            next_file_number: 0,
        }
    }

    pub fn load_existing_dtables(&mut self) {
        let mut lt = self.largetable.write().unwrap();
        for entry in glob::glob(format!("{}/*.{}", self.data_directory, DTABLE_EXT).as_str())
            .expect("Failed to read glob pattern!")
        {
            match entry {
                Ok(path) => {
                    println!("adding dtable: {:?}", &path);
                    let f = fs::File::open(path).unwrap();
                    lt.add_dtable(Box::new(f));
                }
                Err(e) => panic!("{:?}", e),
            }
        }
    }

    pub fn load_existing_journals(&mut self) {
        let mut lt = self.largetable.write().unwrap();
        for entry in glob::glob(format!("{}/*.{}", self.data_directory, JOURNAL_EXT).as_str())
            .expect("Failed to read glob pattern!")
        {
            match entry {
                Ok(path) => {
                    println!("adding journal: {:?}", &path);
                    let f = fs::File::open(path).unwrap();
                    lt.load_from_journal(Box::new(f));
                }
                Err(e) => panic!("{:?}", e),
            }
        }
    }

    pub fn add_journal(&mut self) {
        // First, open the file as writable and dump the mtable to it.
        let f = self.get_new_filehandle(JOURNAL_EXT);
        self.largetable.write().unwrap().add_journal(Box::new(f));
    }

    fn filename_format(&self, file_number: u64, filetype: &str) -> String {
        format!(
            "{}/data-{:04}.{}",
            self.data_directory, file_number, filetype
        )
    }

    fn get_current_filename(&self, filetype: &str) -> String {
        self.filename_format(self.next_file_number, filetype)
    }

    // get_new_filehandle finds a fresh filehandle to use. It keeps incrementing the file
    // counter until it finds a file which doesn't exist yet, and uses that.
    fn get_new_filehandle(&mut self, filetype: &str) -> fs::File {
        // First, check if the file exists.
        let mut filename: String;
        loop {
            filename = self.get_current_filename(filetype);
            if path::Path::new(filename.as_str()).exists() {
                self.next_file_number += 1;
            } else {
                break;
            }
        }
        println!("create file: '{}'", filename);
        fs::File::create(filename).unwrap()
    }

    // check_memory checks the current memory usage and dumps the mtable to disk if necessary.
    pub fn check_memory(&mut self) {
        let memory_usage = { self.largetable.read().unwrap().get_memory_usage() };
        println!(
            "memory check: {} out of {}",
            memory_usage, self.memory_limit
        );
        if memory_usage > self.memory_limit {
            // First, open the file as writable and dump the mtable to it.
            let mut f = self.get_new_filehandle(DTABLE_EXT);
            let mut lt = self.largetable.write().unwrap();
            lt.write_to_disk(&mut f);
            f.flush().unwrap();

            // Now, re-open the file in read mode and construct a dtable from it.
            let f = fs::File::open(self.get_current_filename(DTABLE_EXT)).unwrap();
            lt.add_dtable(Box::new(f));
        }
    }
}

fn get_timestamp_usec() -> u64 {
    let tm = time::now_utc().to_timespec();
    (tm.sec as u64) * 1_000_000 + ((tm.nsec / 1000) as u64)
}

impl largetable_grpc_rust::LargeTableService for LargeTableServiceHandler {
    fn read(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::ReadRequest,
    ) -> grpc::SingleResponse<largetable_grpc_rust::ReadResponse> {
        let timestamp = match req.get_timestamp() {
            0 => get_timestamp_usec(),
            x => x,
        };

        let response =
            match self
                .largetable
                .read()
                .unwrap()
                .read(req.get_row(), req.get_column(), timestamp)
            {
                Some(mut result) => {
                    let mut response = largetable_grpc_rust::ReadResponse::new();
                    response.set_found(!result.deleted);
                    if !result.deleted {
                        response.set_data(result.take_data());
                    }
                    response.set_timestamp(result.get_timestamp());
                    response
                }
                None => {
                    let mut response = largetable_grpc_rust::ReadResponse::new();
                    response.set_found(false);
                    response
                }
            };

        grpc::SingleResponse::completed(response)
    }

    fn read_range(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::ReadRangeRequest,
    ) -> grpc::SingleResponse<largetable_grpc_rust::ReadRangeResponse> {
        let time_usec = if req.get_timestamp() > 0 {
            req.get_timestamp()
        } else {
            get_timestamp_usec()
        };

        let records = self.largetable.read().unwrap().read_range(
            req.get_row(),
            req.get_column_spec(),
            req.get_column_min(),
            req.get_column_max(),
            req.get_max_records(),
            time_usec,
        );

        let mut response = largetable_grpc_rust::ReadRangeResponse::new();

        // Construct the largetable_grpc_rust::Record from the largetable::Record.
        for record in records {
            let mut r = largetable_grpc_rust::Record::new();
            r.set_column(record.get_col().to_string());
            r.set_row(record.get_row().to_string());
            r.set_data(record.get_data().to_owned());
            r.set_timestamp(record.get_timestamp());

            response.mut_records().push(r);
        }

        grpc::SingleResponse::completed(response)
    }

    fn reserve_id(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::ReserveIDRequest,
    ) -> grpc::SingleResponse<largetable_grpc_rust::ReserveIDResponse> {
        let reserved_id = self
            .largetable
            .read()
            .unwrap()
            .reserve_id(req.get_row(), req.get_column());

        let mut response = largetable_grpc_rust::ReserveIDResponse::new();
        response.set_id(reserved_id);

        grpc::SingleResponse::completed(response)
    }

    fn write(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::WriteRequest,
    ) -> grpc::SingleResponse<largetable_grpc_rust::WriteResponse> {
        let time_usec = match req.get_timestamp() {
            0 => get_timestamp_usec(),
            x => x,
        };
        let mut rec = largetable::Record::new();
        rec.set_timestamp(time_usec);
        rec.set_data(req.get_data().to_owned());
        rec.set_deleted(false);

        self.largetable
            .read()
            .unwrap()
            .write(req.get_row(), req.get_column(), rec);

        let mut response = largetable_grpc_rust::WriteResponse::new();
        response.set_timestamp(time_usec);
        grpc::SingleResponse::completed(response)
    }

    fn delete(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::DeleteRequest,
    ) -> grpc::SingleResponse<largetable_grpc_rust::DeleteResponse> {
        let time_usec = get_timestamp_usec();
        let mut rec = largetable::Record::new();
        rec.set_timestamp(time_usec);
        rec.set_deleted(true);
        self.largetable
            .read()
            .unwrap()
            .write(req.get_row(), req.get_column(), rec);

        let mut response = largetable_grpc_rust::DeleteResponse::new();
        response.set_timestamp(time_usec);
        grpc::SingleResponse::completed(response)
    }

    fn get_shard_hint(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::ShardHintRequest,
    ) -> grpc::SingleResponse<largetable_grpc_rust::ShardHintResponse> {
        let shards = self.largetable.read().unwrap().get_shard_hint(
            req.get_row(),
            req.get_column_spec(),
            req.get_column_min(),
            req.get_column_max(),
        );

        let mut hint = largetable_grpc_rust::ShardHintResponse::new();
        hint.set_shards(protobuf::RepeatedField::from_vec(shards));
        grpc::SingleResponse::completed(hint)
    }
}

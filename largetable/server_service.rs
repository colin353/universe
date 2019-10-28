/*
 * serviceimpl.rs
 *
 * This file implements the largetable service.
 */

use largetable;
use largetable_grpc_rust;
use largetable_proto_rust;

use glob;
use protobuf;
use protobuf::Message;
use sstable;
use time;

use std::fs;
use std::io::Write;
use std::path;
use std::sync::{Arc, Mutex, RwLock};

const DTABLE_EXT: &'static str = "sstable";
const JOURNAL_EXT: &'static str = "recordio";
const COMPACTION_POLICIES: &'static str = "__META__POLICIES__";

#[derive(Clone)]
pub struct LargeTableServiceHandler {
    largetable: Arc<RwLock<largetable::LargeTable>>,
    memory_limit: u64,
    data_directory: String,
    next_file_number: Arc<Mutex<u64>>,
    journals: Arc<Mutex<Vec<String>>>,
    dtables: Arc<Mutex<Vec<String>>>,
}

impl LargeTableServiceHandler {
    pub fn new(memory_limit: u64, data_directory: String) -> LargeTableServiceHandler {
        LargeTableServiceHandler {
            largetable: Arc::new(RwLock::new(largetable::LargeTable::new())),
            memory_limit,
            data_directory,
            next_file_number: Arc::new(Mutex::new(0)),
            journals: Arc::new(Mutex::new(Vec::new())),
            dtables: Arc::new(Mutex::new(Vec::new())),
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
                    self.dtables
                        .lock()
                        .unwrap()
                        .push(path.to_str().unwrap().to_owned());
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
                    self.journals
                        .lock()
                        .unwrap()
                        .push(path.to_str().unwrap().to_owned());
                    let f = fs::File::open(path).unwrap();
                    lt.load_from_journal(Box::new(f));
                }
                Err(e) => panic!("{:?}", e),
            }
        }
    }

    pub fn add_journal(&mut self) {
        // First, open the file as writable and dump the mtable to it.
        let (filename, f) = self.get_new_filehandle(JOURNAL_EXT);
        self.journals.lock().unwrap().push(filename);
        self.largetable.write().unwrap().add_journal(Box::new(f));
    }

    pub fn unlink_journals(&mut self, journals: Vec<String>) {
        for filename in journals {
            println!("deleting: {}", filename);
            fs::remove_file(filename).unwrap();
        }
    }

    fn filename_format(&self, file_number: u64, filetype: &str) -> String {
        format!(
            "{}/data-{:04}.{}",
            self.data_directory, file_number, filetype
        )
    }

    fn get_current_filename(&self, filetype: &str) -> String {
        self.filename_format(*self.next_file_number.lock().unwrap(), filetype)
    }

    // get_new_filehandle finds a fresh filehandle to use. It keeps incrementing the file
    // counter until it finds a file which doesn't exist yet, and uses that.
    fn get_new_filehandle(&mut self, filetype: &str) -> (String, fs::File) {
        // First, check if the file exists.
        let mut filename: String;
        loop {
            filename = self.get_current_filename(filetype);
            if path::Path::new(filename.as_str()).exists() {
                *self.next_file_number.lock().unwrap() += 1;
            } else {
                break;
            }
        }
        println!("create file: '{}'", filename);
        (filename.clone(), fs::File::create(filename).unwrap())
    }

    // check_memory checks the current memory usage and dumps the mtable to disk if necessary.
    pub fn check_memory(&mut self) {
        let memory_usage = { self.largetable.read().unwrap().get_memory_usage() };
        println!(
            "memory check: {} out of {}",
            memory_usage, self.memory_limit
        );
        if memory_usage > self.memory_limit {
            // Add a new journal file
            let journals = self.journals.lock().unwrap().clone();
            *self.journals.lock().unwrap() = Vec::new();
            self.add_journal();

            // Create a new mtable
            self.largetable.write().unwrap().add_mtable();

            // Open the file as writable and dump the mtable to it.
            {
                let (_, mut f) = self.get_new_filehandle(DTABLE_EXT);
                let lt = self.largetable.read().unwrap();
                lt.write_to_disk(&mut f, 1);
                f.flush().unwrap();
            }

            // Now, re-open the file in read mode and construct a dtable from it, and
            // simultaneously drop the mtable.
            {
                let filename = self.get_current_filename(DTABLE_EXT);
                let f = fs::File::open(filename.clone()).unwrap();
                self.dtables.lock().unwrap().push(filename);
                let mut lt = self.largetable.write().unwrap();
                lt.add_dtable(Box::new(f));
                lt.drop_mtables();
            }

            // Finally, delete the old journals which have been persisted to disk
            self.unlink_journals(journals);
        }
    }

    pub fn check_compaction(&mut self) {
        if self.dtables.lock().unwrap().len() > 1 {
            self.perform_compaction();
        }
    }

    pub fn perform_compaction(&mut self) {
        let records =
            self.largetable
                .read()
                .unwrap()
                .read_range(COMPACTION_POLICIES, "", "", "", 0, 0);
        let mut policies = Vec::new();
        for record in records {
            let mut p = largetable_grpc_rust::CompactionPolicy::new();
            p.merge_from_bytes(record.get_data()).unwrap();
            policies.push(p);
        }

        let tables_to_replace = self.dtables.lock().unwrap().clone();
        let tables = tables_to_replace
            .iter()
            .map(|filename| {
                let f = fs::File::open(filename).unwrap();
                sstable::SSTableReader::new(Box::new(f)).unwrap()
            })
            .collect();

        let (filename, mut f) = self.get_new_filehandle(DTABLE_EXT);

        println!(
            "compacting {:?} --> {}",
            self.dtables.lock().unwrap(),
            filename
        );

        let mut builder = sstable::SSTableBuilder::new(&mut f);
        compaction::compact(policies, tables, get_timestamp_usec(), &mut builder);

        {
            // Replace the old dtables with the new compacted one
            *self.dtables.lock().unwrap() = vec![filename.clone()];
            let reader = fs::File::open(filename).unwrap();
            let mut lt = self.largetable.write().unwrap();
            lt.clear_dtables();
            lt.add_dtable(Box::new(reader));
        }

        // Delete the old dtable files
        for filename in tables_to_replace {
            fs::remove_file(filename).unwrap();
        }

        println!("compaction complete");
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

    fn batch_read(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::BatchReadRequest,
    ) -> grpc::SingleResponse<largetable_grpc_rust::BatchReadResponse> {
        grpc::SingleResponse::completed(largetable_grpc_rust::BatchReadResponse::new())
    }

    fn batch_write(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::BatchWriteRequest,
    ) -> grpc::SingleResponse<largetable_grpc_rust::WriteResponse> {
        let request_time = get_timestamp_usec();
        {
            let table = self.largetable.read().unwrap();
            for write in req.get_writes() {
                let time_usec = match write.get_timestamp() {
                    0 => request_time,
                    x => x,
                };
                let mut rec = largetable::Record::new();
                rec.set_timestamp(time_usec);
                rec.set_data(write.get_data().to_owned());
                rec.set_deleted(false);

                table.write(write.get_row(), write.get_column(), rec);
            }

            for delete in req.get_deletes() {
                let mut rec = largetable::Record::new();
                rec.set_timestamp(request_time);
                rec.set_deleted(true);

                table.write(delete.get_row(), delete.get_column(), rec);
            }
        }

        let mut response = largetable_grpc_rust::WriteResponse::new();
        response.set_timestamp(request_time);
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

    fn set_compaction_policy(
        &self,
        _m: grpc::RequestOptions,
        req: largetable_grpc_rust::CompactionPolicy,
    ) -> grpc::SingleResponse<largetable_grpc_rust::SetCompactionPolicyResponse> {
        let mut rec = largetable_proto_rust::Record::new();
        req.write_to_vec(&mut rec.mut_data());
        self.largetable.read().unwrap().write(
            COMPACTION_POLICIES,
            &format!("{}_{}", req.get_row(), req.get_scope()),
            rec,
        );

        grpc::SingleResponse::completed(largetable_grpc_rust::SetCompactionPolicyResponse::new())
    }
}

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
use sstable::{SSTableBuilder, SSTableReader};
use time;

use std::fs;
use std::io::{BufReader, Write};
use std::path;
use std::sync::{Arc, Mutex, RwLock};

const DTABLE_EXT: &'static str = "sstable";
const DTABLE_TEMPORARY_EXT: &'static str = "sstable-temporary";
const JOURNAL_EXT: &'static str = "recordio";
const COMPACTION_POLICIES: &'static str = "__META__POLICIES__";
const BUFFER_SIZE: usize = 64000;

#[derive(Clone)]
pub struct LargeTableServiceHandler {
    largetable: Arc<RwLock<largetable::LargeTable>>,
    memory_limit: u64,
    data_directory: String,
    next_file_number: Arc<Mutex<u64>>,
    journals: Arc<Mutex<Vec<String>>>,
    dtables: Arc<Mutex<Vec<String>>>,
    logger: logger_client::LoggerClient,

    // The server may have some startup business to complete before
    // it can respond to requests. However, it should still accept the
    // requests, and just have a delay before responding.
    // Requests should double check that we are in the ready state before
    // responding.
    ready: Arc<RwLock<bool>>,
}

impl LargeTableServiceHandler {
    pub fn new(
        memory_limit: u64,
        data_directory: String,
        logger: logger_client::LoggerClient,
    ) -> Self {
        LargeTableServiceHandler {
            largetable: Arc::new(RwLock::new(largetable::LargeTable::new())),
            memory_limit,
            data_directory,
            next_file_number: Arc::new(Mutex::new(0)),
            journals: Arc::new(Mutex::new(Vec::new())),
            dtables: Arc::new(Mutex::new(Vec::new())),
            ready: Arc::new(RwLock::new(false)),
            logger: logger,
        }
    }

    pub fn ready(&self) {
        *self.ready.write().unwrap() = true;
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
                    lt.add_dtable(fs::File::open(&path).unwrap());
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
                    let f = BufReader::with_capacity(BUFFER_SIZE, fs::File::open(path).unwrap());
                    lt.load_from_journal(Box::new(f));
                }
                Err(e) => panic!("{:?}", e),
            }
        }
    }

    pub fn add_journal(&mut self) {
        // First, open the file as writable and dump the mtable to it.
        let filename = self.get_new_filename(JOURNAL_EXT);
        let f = std::fs::File::create(&filename).unwrap();
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

    // get_new_filename finds a fresh filename to use. It keeps incrementing the file
    // counter until it finds a file which doesn't exist yet, and uses that.
    fn get_new_filename(&mut self, filetype: &str) -> String {
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
        filename
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
                let filename = self.get_new_filename(DTABLE_EXT);
                let mut f = fs::File::create(filename).unwrap();
                let lt = self.largetable.read().unwrap();
                lt.write_to_disk(&mut f, 1);
                f.flush().unwrap();
            }

            // Now, re-open the file in read mode and construct a dtable from it, and
            // simultaneously drop the mtable.
            {
                let filename = self.get_current_filename(DTABLE_EXT);
                self.dtables.lock().unwrap().push(filename.clone());

                let mut lt = self.largetable.write().unwrap();
                lt.add_dtable(std::fs::File::open(&filename).unwrap());
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

        // Metapolicy to clean up compaction policies!
        let mut metapolicy = largetable_grpc_rust::CompactionPolicy::new();
        metapolicy.set_row(COMPACTION_POLICIES.into());
        metapolicy.set_history(1);
        policies.push(metapolicy);

        for record in records {
            let mut p = largetable_grpc_rust::CompactionPolicy::new();
            p.merge_from_bytes(record.get_data()).unwrap();
            policies.push(p);
        }

        let tables_to_replace = self.dtables.lock().unwrap().clone();
        let tables = tables_to_replace
            .iter()
            .map(|filename| SSTableReader::new(std::fs::File::open(filename).unwrap()).unwrap())
            .collect();

        // First allocate a temporary filename. We don't want to use the sstable filename
        // in case we get a crash during compaction.
        let tmp_filename = self.get_new_filename(DTABLE_TEMPORARY_EXT);
        let mut f = std::fs::File::create(&tmp_filename).unwrap();

        println!(
            "compacting {:?} --> {}",
            self.dtables.lock().unwrap(),
            tmp_filename
        );

        let builder = SSTableBuilder::new(&mut f);
        compaction::compact(policies, tables, get_timestamp_usec(), builder);

        // Now that compaction is done, rename the temporary file to a new filename.
        let filename = self.get_new_filename(DTABLE_EXT);
        std::fs::rename(&tmp_filename, &filename).unwrap();

        {
            // Replace the old dtables with the new compacted one
            *self.dtables.lock().unwrap() = vec![filename.clone()];
            let mut lt = self.largetable.write().unwrap();
            lt.clear_dtables();
            lt.add_dtable(std::fs::File::open(&filename).unwrap());
        }

        // Delete the old dtable files
        for filename in tables_to_replace {
            fs::remove_file(filename).unwrap();
        }

        println!("compaction complete");
    }

    fn wait_until_ready(&self) {
        while !*self.ready.read().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(100));
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
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<largetable_grpc_rust::ReadRequest>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::ReadResponse>,
    ) -> grpc::Result<()> {
        let start = std::time::Instant::now();
        self.wait_until_ready();

        let timestamp = match req.message.get_timestamp() {
            0 => get_timestamp_usec(),
            x => x,
        };

        let response = match self.largetable.read().unwrap().read(
            req.message.get_row(),
            req.message.get_column(),
            timestamp,
        ) {
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

        let mut pl = logger_client::LargetablePerfLog::new();
        pl.set_row(req.message.get_row().to_string());
        pl.set_records(if response.get_found() { 1 } else { 0 });
        pl.set_request_duration_micros(start.elapsed().as_micros() as u64);
        pl.set_kind(logger_client::ReadKind::READ);
        pl.set_size_bytes(response.get_data().len() as u64);
        self.logger.log(logger_client::Log::LARGETABLE_READS, &pl);

        resp.finish(response)
    }

    fn read_range(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<largetable_grpc_rust::ReadRangeRequest>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::ReadRangeResponse>,
    ) -> grpc::Result<()> {
        let start = std::time::Instant::now();
        self.wait_until_ready();

        let time_usec = if req.message.get_timestamp() > 0 {
            req.message.get_timestamp()
        } else {
            get_timestamp_usec()
        };

        let records = self.largetable.read().unwrap().read_range(
            req.message.get_row(),
            req.message.get_column_spec(),
            req.message.get_column_min(),
            req.message.get_column_max(),
            req.message.get_max_records(),
            time_usec,
        );

        let mut response = largetable_grpc_rust::ReadRangeResponse::new();

        // Construct the largetable_grpc_rust::Record from the largetable::Record.
        let mut size_bytes = 0;
        for record in records {
            size_bytes += record.get_data().len();

            let mut r = largetable_grpc_rust::Record::new();
            r.set_column(record.get_col().to_string());
            r.set_row(record.get_row().to_string());
            r.set_data(record.get_data().to_owned());
            r.set_timestamp(record.get_timestamp());

            response.mut_records().push(r);
        }

        let mut pl = logger_client::LargetablePerfLog::new();
        pl.set_row(req.message.get_row().to_string());
        pl.set_records(response.get_records().len() as u64);
        pl.set_request_duration_micros(start.elapsed().as_micros() as u64);
        pl.set_kind(logger_client::ReadKind::READ_RANGE);
        pl.set_size_bytes(size_bytes as u64);
        self.logger.log(logger_client::Log::LARGETABLE_READS, &pl);

        resp.finish(response)
    }

    fn reserve_id(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<largetable_grpc_rust::ReserveIDRequest>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::ReserveIDResponse>,
    ) -> grpc::Result<()> {
        self.wait_until_ready();

        let reserved_id = self
            .largetable
            .read()
            .unwrap()
            .reserve_id(req.message.get_row(), req.message.get_column());

        let mut response = largetable_grpc_rust::ReserveIDResponse::new();
        response.set_id(reserved_id);

        resp.finish(response)
    }

    fn write(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<largetable_grpc_rust::WriteRequest>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::WriteResponse>,
    ) -> grpc::Result<()> {
        let start = std::time::Instant::now();
        self.wait_until_ready();

        let time_usec = match req.message.get_timestamp() {
            0 => get_timestamp_usec(),
            x => x,
        };
        let size_bytes = req.message.get_data().len();

        let mut rec = largetable::Record::new();
        rec.set_timestamp(time_usec);
        rec.set_data(req.message.get_data().to_owned());
        rec.set_deleted(false);

        self.largetable
            .read()
            .unwrap()
            .write(req.message.get_row(), req.message.get_column(), rec);

        let mut response = largetable_grpc_rust::WriteResponse::new();
        response.set_timestamp(time_usec);

        let mut pl = logger_client::LargetablePerfLog::new();
        pl.set_row(req.message.get_row().to_string());
        pl.set_records(1);
        pl.set_request_duration_micros(start.elapsed().as_micros() as u64);
        pl.set_kind(logger_client::ReadKind::WRITE);
        pl.set_size_bytes(size_bytes as u64);
        self.logger.log(logger_client::Log::LARGETABLE_READS, &pl);

        resp.finish(response)
    }

    fn delete(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<largetable_grpc_rust::DeleteRequest>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::DeleteResponse>,
    ) -> grpc::Result<()> {
        self.wait_until_ready();

        let time_usec = get_timestamp_usec();
        let mut rec = largetable::Record::new();
        rec.set_timestamp(time_usec);
        rec.set_deleted(true);
        self.largetable
            .read()
            .unwrap()
            .write(req.message.get_row(), req.message.get_column(), rec);

        let mut response = largetable_grpc_rust::DeleteResponse::new();
        response.set_timestamp(time_usec);
        resp.finish(response)
    }

    fn batch_read(
        &self,
        _: grpc::ServerHandlerContext,
        _req: grpc::ServerRequestSingle<largetable_grpc_rust::BatchReadRequest>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::BatchReadResponse>,
    ) -> grpc::Result<()> {
        self.wait_until_ready();
        resp.finish(largetable_grpc_rust::BatchReadResponse::new())
    }

    fn batch_write(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<largetable_grpc_rust::BatchWriteRequest>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::WriteResponse>,
    ) -> grpc::Result<()> {
        let start = std::time::Instant::now();
        self.wait_until_ready();
        let mut size_bytes = 0;

        let request_time = get_timestamp_usec();
        {
            let table = self.largetable.read().unwrap();
            for write in req.message.get_writes() {
                let time_usec = match write.get_timestamp() {
                    0 => request_time,
                    x => x,
                };
                size_bytes += write.get_data().len();
                let mut rec = largetable::Record::new();
                rec.set_timestamp(time_usec);
                rec.set_data(write.get_data().to_owned());
                rec.set_deleted(false);

                table.write(write.get_row(), write.get_column(), rec);
            }

            for delete in req.message.get_deletes() {
                let mut rec = largetable::Record::new();
                rec.set_timestamp(request_time);
                rec.set_deleted(true);

                table.write(delete.get_row(), delete.get_column(), rec);
            }
        }

        let mut pl = logger_client::LargetablePerfLog::new();
        pl.set_records((req.message.get_writes().len() + req.message.get_deletes().len()) as u64);
        pl.set_request_duration_micros(start.elapsed().as_micros() as u64);
        pl.set_kind(logger_client::ReadKind::BULK_WRITE);
        pl.set_size_bytes(size_bytes as u64);
        self.logger.log(logger_client::Log::LARGETABLE_READS, &pl);

        let mut response = largetable_grpc_rust::WriteResponse::new();
        response.set_timestamp(request_time);
        resp.finish(response)
    }

    fn get_shard_hint(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<largetable_grpc_rust::ShardHintRequest>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::ShardHintResponse>,
    ) -> grpc::Result<()> {
        self.wait_until_ready();

        let shards = self.largetable.read().unwrap().get_shard_hint(
            req.message.get_row(),
            req.message.get_column_spec(),
            req.message.get_column_min(),
            req.message.get_column_max(),
        );

        let mut hint = largetable_grpc_rust::ShardHintResponse::new();
        hint.set_shards(protobuf::RepeatedField::from_vec(shards));
        resp.finish(hint)
    }

    fn set_compaction_policy(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<largetable_grpc_rust::CompactionPolicy>,
        resp: grpc::ServerResponseUnarySink<largetable_grpc_rust::SetCompactionPolicyResponse>,
    ) -> grpc::Result<()> {
        self.wait_until_ready();

        let mut rec = largetable_proto_rust::Record::new();
        req.message.write_to_vec(&mut rec.mut_data()).unwrap();
        self.largetable.read().unwrap().write(
            COMPACTION_POLICIES,
            &format!("{}_{}", req.message.get_row(), req.message.get_scope()),
            rec,
        );

        resp.finish(largetable_grpc_rust::SetCompactionPolicyResponse::new())
    }
}

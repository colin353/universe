extern crate futures;
extern crate grpc;
extern crate largetable_grpc_rust;
extern crate protobuf;

mod client_service;

pub use largetable_grpc_rust::{
    BatchReadRequest, BatchReadResponse, BatchWriteRequest, CompactionPolicy, DeleteResponse,
    ReadRangeResponse, ReadResponse, Record, ShardHintResponse, WriteResponse,
};

use largetable_grpc_rust::LargeTableService;

use std::sync::Arc;

pub trait LargeTableClient {
    fn write(
        &self,
        row: &str,
        col: &str,
        timestamp: u64,
        data: Vec<u8>,
    ) -> largetable_grpc_rust::WriteResponse;
    fn delete(&self, row: &str, col: &str) -> largetable_grpc_rust::DeleteResponse;
    fn read(&self, row: &str, col: &str, timestamp: u64) -> largetable_grpc_rust::ReadResponse;
    fn reserve_id(&self, row: &str, col: &str) -> u64;

    fn read_range(
        &self,
        row: &str,
        min_col: &str,
        max_col: &str,
        limit: u64,
        timestamp: u64,
    ) -> largetable_grpc_rust::ReadRangeResponse;

    fn read_scoped(
        &self,
        row: &str,
        col_spec: &str,
        min_col: &str,
        max_col: &str,
        limit: u64,
        timestamp: u64,
    ) -> largetable_grpc_rust::ReadRangeResponse;

    fn set_compaction_policy(&self, policy: largetable_grpc_rust::CompactionPolicy);

    fn shard_hint(&self, row: &str, col_spec: &str) -> largetable_grpc_rust::ShardHintResponse;

    fn batch_write(
        &self,
        largetable_grpc_rust::BatchWriteRequest,
    ) -> largetable_grpc_rust::WriteResponse;

    fn batch_read(
        &self,
        req: largetable_grpc_rust::BatchReadRequest,
    ) -> largetable_grpc_rust::BatchReadResponse;

    fn read_proto<T: protobuf::Message>(&self, row: &str, col: &str, timestamp: u64) -> Option<T> {
        let response = self.read(row, col, timestamp);
        let mut message = T::new();
        if !response.get_found() {
            return None;
        }

        message
            .merge_from_bytes(response.get_data())
            .expect("unable to deserialize proto");
        Some(message)
    }

    fn write_proto<T: protobuf::Message>(
        &self,
        row: &str,
        col: &str,
        timestamp: u64,
        message: &T,
    ) -> largetable_grpc_rust::WriteResponse {
        let mut message_bytes = Vec::new();
        message
            .write_to_vec(&mut message_bytes)
            .expect("unable to serialize message");

        self.write(row, col, timestamp, message_bytes)
    }
}

pub struct LargeTableRemoteClient {
    hostname: String,
    port: u16,
    client: largetable_grpc_rust::LargeTableServiceClient,
}

impl LargeTableRemoteClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        LargeTableRemoteClient {
            hostname: hostname.to_owned(),
            port: port,
            client: largetable_grpc_rust::LargeTableServiceClient::new_plain(
                hostname,
                port,
                Default::default(),
            )
            .unwrap(),
        }
    }

    fn opts(&self) -> grpc::RequestOptions {
        grpc::RequestOptions::new()
    }
}

impl Clone for LargeTableRemoteClient {
    fn clone(&self) -> LargeTableRemoteClient {
        LargeTableRemoteClient::new(&self.hostname, self.port)
    }
}

pub struct LargeTableBatchWriter {
    req: largetable_grpc_rust::BatchWriteRequest,
}

impl LargeTableBatchWriter {
    pub fn new() -> Self {
        Self {
            req: largetable_grpc_rust::BatchWriteRequest::new(),
        }
    }

    pub fn write(&mut self, row: &str, col: &str, timestamp: u64, data: Vec<u8>) {
        let mut write = largetable_grpc_rust::WriteRequest::new();
        write.set_row(row.to_owned());
        write.set_column(col.to_owned());
        write.set_timestamp(timestamp);
        write.set_data(data);

        self.req.mut_writes().push(write);
    }

    pub fn delete(&mut self, row: &str, col: &str) {
        let mut delete = largetable_grpc_rust::DeleteRequest::new();
        delete.set_row(row.to_owned());
        delete.set_column(col.to_owned());

        self.req.mut_deletes().push(delete);
    }

    pub fn write_proto<T: protobuf::Message>(
        &mut self,
        row: &str,
        col: &str,
        timestamp: u64,
        message: &T,
    ) {
        let mut message_bytes = Vec::new();
        message
            .write_to_vec(&mut message_bytes)
            .expect("unable to serialize message");
        self.write(row, col, timestamp, message_bytes)
    }

    pub fn finish<C: LargeTableClient>(self, client: &C) -> largetable_grpc_rust::WriteResponse {
        if self.req.get_writes().len() > 0 || self.req.get_deletes().len() > 0 {
            client.batch_write(self.req)
        } else {
            largetable_grpc_rust::WriteResponse::new()
        }
    }
}

pub struct LargeTableScopedIterator<'a, T: protobuf::Message, C: LargeTableClient> {
    client: &'a C,
    marker: std::marker::PhantomData<T>,
    buffer: Vec<(String, T)>,
    finished: bool,

    // Query details
    row: String,
    col_spec: String,
    min_col: String,
    max_col: String,
    timestamp: u64,

    // Number of result to get per query.
    pub batch_size: u64,
}

impl<'a, T: protobuf::Message, C: LargeTableClient> LargeTableScopedIterator<'a, T, C> {
    pub fn new(
        client: &'a C,
        row: String,
        col_spec: String,
        min_col: String,
        max_col: String,
        timestamp: u64,
    ) -> Self {
        LargeTableScopedIterator {
            client: client,
            marker: std::marker::PhantomData,
            buffer: Vec::new(),
            finished: false,
            row: row,
            col_spec: col_spec,
            min_col: min_col,
            max_col: max_col,
            timestamp: timestamp,
            batch_size: 1024,
        }
    }
}

impl<'a, T: protobuf::Message, C: LargeTableClient> Iterator
    for LargeTableScopedIterator<'a, T, C>
{
    type Item = (String, T);
    fn next(&mut self) -> Option<(String, T)> {
        // If we have any data, return that. If there's just one record left, then we'll use that
        // as the min_col value and make another call to the server.
        if self.buffer.len() > 1 || self.finished {
            return self.buffer.pop();
        } else if let Some((col, _)) = self.buffer.pop() {
            self.min_col = col;
        }

        // Run the query and get some data to fill the buffer.
        let response = self.client.read_scoped(
            &self.row,
            &self.col_spec,
            &self.min_col,
            &self.max_col,
            self.batch_size,
            self.timestamp,
        );

        if (response.get_records().len() as u64) < self.batch_size {
            self.finished = true;
        }

        // Iterate in reverse order. That way, when we pop from the vector
        // we get the records in alphabetical order.
        for record in response.get_records().iter().rev() {
            let mut message = T::new();
            message
                .merge_from_bytes(record.get_data())
                .expect("unable to deserialize proto");
            self.buffer.push((record.get_column().to_owned(), message));
        }

        self.buffer.pop()
    }
}

impl LargeTableClient for LargeTableRemoteClient {
    fn write(
        &self,
        row: &str,
        col: &str,
        timestamp: u64,
        data: Vec<u8>,
    ) -> largetable_grpc_rust::WriteResponse {
        let mut req = largetable_grpc_rust::WriteRequest::new();
        req.set_row(row.to_owned());
        req.set_column(col.to_owned());
        req.set_data(data);
        req.set_timestamp(timestamp);
        self.client.write(self.opts(), req).wait().expect("rpc").1
    }

    fn delete(&self, row: &str, col: &str) -> largetable_grpc_rust::DeleteResponse {
        let mut req = largetable_grpc_rust::DeleteRequest::new();
        req.set_row(row.to_owned());
        req.set_column(col.to_owned());
        self.client.delete(self.opts(), req).wait().expect("rpc").1
    }

    fn read(&self, row: &str, col: &str, timestamp: u64) -> largetable_grpc_rust::ReadResponse {
        let mut req = largetable_grpc_rust::ReadRequest::new();
        req.set_row(row.to_owned());
        req.set_column(col.to_owned());
        req.set_timestamp(timestamp);
        self.client.read(self.opts(), req).wait().expect("rpc").1
    }

    fn set_compaction_policy(&self, policy: largetable_grpc_rust::CompactionPolicy) {
        self.client
            .set_compaction_policy(self.opts(), policy)
            .wait()
            .expect("rpc");
    }

    fn reserve_id(&self, row: &str, col: &str) -> u64 {
        let mut req = largetable_grpc_rust::ReserveIDRequest::new();
        req.set_row(row.to_owned());
        req.set_column(col.to_owned());

        let response = self
            .client
            .reserve_id(self.opts(), req)
            .wait()
            .expect("rpc")
            .1;
        response.get_id()
    }

    fn read_range(
        &self,
        row: &str,
        min_col: &str,
        max_col: &str,
        limit: u64,
        timestamp: u64,
    ) -> largetable_grpc_rust::ReadRangeResponse {
        let mut req = largetable_grpc_rust::ReadRangeRequest::new();
        req.set_row(row.to_owned());
        req.set_column_min(min_col.to_owned());
        req.set_column_max(max_col.to_owned());
        req.set_max_records(limit);
        req.set_timestamp(timestamp);

        self.client
            .read_range(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    fn read_scoped(
        &self,
        row: &str,
        col_spec: &str,
        min_col: &str,
        max_col: &str,
        limit: u64,
        timestamp: u64,
    ) -> largetable_grpc_rust::ReadRangeResponse {
        let mut req = largetable_grpc_rust::ReadRangeRequest::new();
        req.set_row(row.to_owned());
        req.set_column_min(min_col.to_owned());
        req.set_column_max(max_col.to_owned());
        req.set_column_spec(col_spec.to_owned());
        req.set_timestamp(timestamp);
        req.set_max_records(limit);

        self.client
            .read_range(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    fn shard_hint(&self, row: &str, col_spec: &str) -> largetable_grpc_rust::ShardHintResponse {
        let mut req = largetable_grpc_rust::ShardHintRequest::new();
        req.set_row(row.to_owned());
        req.set_column_spec(col_spec.to_owned());

        self.client
            .get_shard_hint(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    fn batch_write(
        &self,
        req: largetable_grpc_rust::BatchWriteRequest,
    ) -> largetable_grpc_rust::WriteResponse {
        self.client
            .batch_write(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    fn batch_read(
        &self,
        req: largetable_grpc_rust::BatchReadRequest,
    ) -> largetable_grpc_rust::BatchReadResponse {
        self.client
            .batch_read(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }
}

pub struct LargeTableBatchReader<T> {
    req: largetable_grpc_rust::BatchReadRequest,
    marker: std::marker::PhantomData<T>,
}

impl<T: protobuf::Message> LargeTableBatchReader<T> {
    pub fn new() -> Self {
        Self {
            req: largetable_grpc_rust::BatchReadRequest::new(),
            marker: std::marker::PhantomData,
        }
    }

    pub fn read(&mut self, row: &str, col: &str, timestamp: u64) -> usize {
        let mut req = largetable_grpc_rust::ReadRequest::new();
        req.set_row(row.to_owned());
        req.set_column(col.to_owned());
        req.set_timestamp(timestamp);
        self.req.mut_reads().push(req);
        self.req.get_reads().len() - 1
    }

    pub fn finish<C: LargeTableClient>(&mut self, client: C) -> Vec<Option<T>> {
        let response = client.batch_read(std::mem::replace(
            &mut self.req,
            largetable_grpc_rust::BatchReadRequest::new(),
        ));
        let mut buffer = Vec::with_capacity(response.get_responses().len());
        for response in response.get_responses() {
            if !response.get_found() {
                buffer.push(None);
                continue;
            }
            let mut message = T::new();
            message
                .merge_from_bytes(response.get_data())
                .expect("unable to deserialize proto");
            buffer.push(Some(message));
        }
        buffer
    }
}

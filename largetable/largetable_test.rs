extern crate protobuf;
extern crate time;

extern crate largetable;
extern crate largetable_client;

use largetable::Record;
use largetable_client::LargeTableBatchWriter;
use largetable_client::LargeTableClient;

use std::sync::Arc;
use std::sync::RwLock;

#[derive(Clone)]
pub struct LargeTableMockClient {
    database: Arc<RwLock<largetable::LargeTable>>,
}

impl LargeTableMockClient {
    pub fn new() -> Self {
        LargeTableMockClient {
            database: Arc::new(RwLock::new(largetable::LargeTable::new())),
        }
    }

    fn read_range_scoped(
        &self,
        row: &str,
        col_spec: &str,
        min_col: &str,
        max_col: &str,
        limit: u64,
        timestamp: u64,
    ) -> largetable_client::ReadRangeResponse {
        let time_usec = if timestamp == 0 {
            get_timestamp_usec()
        } else {
            timestamp
        };

        let records = self
            .database
            .read()
            .unwrap()
            .read_range(row, col_spec, min_col, max_col, limit, time_usec);

        let mut response = largetable_client::ReadRangeResponse::new();

        // Construct the service::Record from the largetable::Record.
        for record in records {
            let mut r = largetable_client::Record::new();
            r.set_column(record.get_col().to_string());
            r.set_row(record.get_row().to_string());
            r.set_data(record.get_data().to_owned());
            r.set_timestamp(record.get_timestamp());

            response.mut_records().push(r);
        }

        response
    }
}

impl LargeTableClient for LargeTableMockClient {
    fn write(
        &self,
        row: &str,
        col: &str,
        timestamp: u64,
        data: Vec<u8>,
    ) -> largetable_client::WriteResponse {
        let ts = match timestamp {
            0 => get_timestamp_usec(),
            x => x,
        };
        let mut record = Record::new();
        record.set_row(row.to_owned());
        record.set_col(col.to_owned());
        record.set_timestamp(ts);
        record.set_data(data);

        self.database.read().unwrap().write(row, col, record);

        let mut response = largetable_client::WriteResponse::new();
        response.set_timestamp(ts);
        response
    }

    fn batch_read(
        &self,
        req: largetable_client::BatchReadRequest,
    ) -> largetable_client::BatchReadResponse {
        largetable_client::BatchReadResponse::new()
    }

    fn batch_write(
        &self,
        req: largetable_client::BatchWriteRequest,
    ) -> largetable_client::WriteResponse {
        let request_time = get_timestamp_usec();
        {
            let table = self.database.read().unwrap();
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
        let mut response = largetable_client::WriteResponse::new();
        response.set_timestamp(request_time);
        response
    }

    fn delete(&self, row: &str, col: &str) -> largetable_client::DeleteResponse {
        let ts = get_timestamp_usec();
        let mut rec = Record::new();
        rec.set_timestamp(ts);
        rec.set_deleted(true);
        self.database.read().unwrap().write(row, col, rec);

        let mut response = largetable_client::DeleteResponse::new();
        response.set_timestamp(ts);
        response
    }

    fn read(&self, row: &str, col: &str, timestamp: u64) -> largetable_client::ReadResponse {
        let timestamp = match timestamp {
            0 => get_timestamp_usec(),
            x => x,
        };

        //println!("[lt] read row:{} col:{} ts:{}", row, col, timestamp);

        match self.database.read().unwrap().read(row, col, timestamp) {
            Some(mut result) => {
                let mut response = largetable_client::ReadResponse::new();
                response.set_found(!result.deleted);
                if !result.deleted {
                    response.set_data(result.take_data());
                }
                response.set_timestamp(result.get_timestamp());
                response
            }
            None => {
                let mut response = largetable_client::ReadResponse::new();
                response.set_found(false);
                response
            }
        }
    }

    fn read_range(
        &self,
        row: &str,
        min_col: &str,
        max_col: &str,
        limit: u64,
        timestamp: u64,
    ) -> largetable_client::ReadRangeResponse {
        self.read_range_scoped(row, "", min_col, max_col, limit, timestamp)
    }

    fn read_scoped(
        &self,
        row: &str,
        col_spec: &str,
        min_col: &str,
        max_col: &str,
        limit: u64,
        timestamp: u64,
    ) -> largetable_client::ReadRangeResponse {
        self.read_range_scoped(row, col_spec, min_col, max_col, limit, timestamp)
    }

    fn shard_hint(&self, row: &str, col_spec: &str) -> largetable_client::ShardHintResponse {
        let shards = self
            .database
            .read()
            .unwrap()
            .get_shard_hint(row, col_spec, "", "");

        let mut hint = largetable_client::ShardHintResponse::new();
        hint.set_shards(protobuf::RepeatedField::from_vec(shards));

        hint
    }

    fn reserve_id(&self, col: &str, row: &str) -> u64 {
        self.database.read().unwrap().reserve_id(col, row)
    }
}

fn get_timestamp_usec() -> u64 {
    let tm = time::now_utc().to_timespec();
    (tm.sec as u64) * 1_000_000 + ((tm.nsec / 1000) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_write() {
        let client = LargeTableMockClient::new();
        client.write("test_row", "test_col", 0, vec![10, 20, 30, 40]);

        let response = client.read("test_row", "test_col", 0);
        assert_eq!(response.get_found(), true);
        assert_eq!(response.get_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn trivial_test() {
        let client = LargeTableMockClient::new();
        client.write("row", "test_col", 0, vec![]);
        client.write("row", "another_col", 0, vec![]);
        client.write("row", "my_other_col", 0, vec![]);

        let iter = largetable_client::LargeTableScopedIterator::<largetable_client::Record, _>::new(
            &client,
            String::from("row"),
            String::from(""),
            String::from(""),
            String::from(""),
            std::u64::MAX,
        );

        assert_eq!(iter.count(), 3);
    }

    #[test]
    fn reserve_id() {
        let client = LargeTableMockClient::new();
        assert_eq!(client.reserve_id("test", "test"), 1);
        assert_eq!(client.reserve_id("test", "test"), 2);
        assert_eq!(client.reserve_id("test", "test"), 3);
    }

    #[test]
    fn test_read_write() {
        let client = LargeTableMockClient::new();
        client.write("test_row", "test_col", 1, vec![10, 20, 30, 40]);

        let response = client.read("test_row", "test_col", 1);
        assert_eq!(response.get_found(), true);
        assert_eq!(response.get_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn test_batch_write() {
        let client = LargeTableMockClient::new();
        let mut bw = LargeTableBatchWriter::new();
        bw.write("test_row", "test_col", 0, vec![10, 20, 30, 40]);
        bw.finish(&client);

        let response = client.read("test_row", "test_col", 0);
        assert_eq!(response.get_found(), true);
        assert_eq!(response.get_data(), &[10, 20, 30, 40]);
    }
}

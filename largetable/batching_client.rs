use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::ops::Bound::{Included, Unbounded};
use std::sync::RwLock;

pub struct LargeTableBatchingClient<T, C: largetable_client::LargeTableClient> {
    cache: RwLock<VecDeque<RwLock<BTreeMap<(String, String), (u64, T)>>>>,
    pub client: C,
}

fn get_timestamp_usec() -> u64 {
    let tm = time::now_utc().to_timespec();
    (tm.sec as u64) * 1_000_000 + ((tm.nsec / 1000) as u64)
}

impl<T: protobuf::Message + Clone, C: largetable_client::LargeTableClient>
    LargeTableBatchingClient<T, C>
{
    pub fn new(client: C) -> Self {
        Self {
            cache: RwLock::new(VecDeque::new()),
            client: client,
        }
    }

    pub fn new_with_cache(client: C) -> Self {
        Self {
            cache: RwLock::new(VecDeque::from(vec![RwLock::new(BTreeMap::new())])),
            client: client,
        }
    }

    pub fn read(&self, row: &str, col: &str) -> Option<T> {
        for cache in self.cache.read().unwrap().iter().rev() {
            if let Some((_, x)) = cache.read().unwrap().get(&(row.to_owned(), col.to_owned())) {
                return Some(x.clone());
            }
        }
        self.client.read_proto::<T>(row, col, 0)
    }

    pub fn write(&self, row: &str, col: &str, message: T) {
        let timestamp = get_timestamp_usec();
        if let Some(cache) = self.cache.read().unwrap().back() {
            let mut writeable_cache = cache.write().unwrap();

            // Need to check whether the record in the cache has a larger timestamp - in which case
            // we just drop the write.
            if let Some((existing_timestamp, _)) =
                writeable_cache.get(&(row.to_owned(), col.to_owned()))
            {
                if *existing_timestamp > timestamp {
                    return;
                }
            }

            writeable_cache.insert((row.to_owned(), col.to_owned()), (timestamp, message));
            return;
        }

        self.client.write_proto::<T>(row, col, timestamp, &message);
    }

    pub fn read_scoped(&self, row: &str, col_spec: &str) -> Vec<T> {
        let mut output = BTreeMap::new();
        let mut response = self.client.read_scoped(row, col_spec, "", "", 1024, 0);
        for record in response.take_records().into_iter() {
            let mut msg = T::new();
            msg.merge_from_bytes(record.get_data());
            output.insert(record.get_column().to_owned(), msg);
        }

        for cache in self.cache.read().unwrap().iter() {
            for ((_row, col), (_, record)) in cache.read().unwrap().range((
                Included((String::from(row), String::from(col_spec))),
                Unbounded,
            )) {
                if _row != row {
                    break;
                }
                if !col.starts_with(col_spec) {
                    break;
                }
                output.insert(col.to_owned(), record.to_owned());
            }
        }

        output.into_iter().map(|(_, r)| r).collect()
    }

    pub fn flush(&self) -> u64 {
        self.prepare_flush();
        let count = self.perform_flush();
        self.finish_flush();
        count
    }

    fn prepare_flush(&self) {
        self.cache
            .write()
            .unwrap()
            .push_back(RwLock::new(BTreeMap::new()));
    }

    fn finish_flush(&self) {
        self.cache.write().unwrap().pop_front();
    }

    fn perform_flush(&self) -> u64 {
        let cache_locked = self.cache.read().unwrap();
        let cache = match cache_locked.front() {
            Some(x) => x,
            None => return 0,
        };

        let mut writer = largetable_client::LargeTableBatchWriter::new();
        let mut count = 0;
        for ((row, col), (timestamp, msg)) in cache.read().unwrap().iter() {
            count += 1;
            writer.write_proto(row, col, *timestamp, msg);
        }
        writer.finish(&self.client);
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

    #[test]
    fn test_write() {
        let client = largetable_test::LargeTableMockClient::new();
        let mut batching_client = LargeTableBatchingClient::new(client);

        batching_client.write("test_row", "test_col", largetable_client::Record::new());
        batching_client.read("test_row", "test_col").unwrap();
    }

    #[test]
    fn test_write_with_cache() {
        let client = largetable_test::LargeTableMockClient::new();
        let mut batching_client = LargeTableBatchingClient::new(client);
        batching_client.write("test_row", "test_col", largetable_client::Record::new());
        batching_client.read("test_row", "test_col").unwrap();
    }

    #[test]
    fn test_write_with_flush() {
        let client = largetable_test::LargeTableMockClient::new();
        let mut batching_client = LargeTableBatchingClient::new(client);
        batching_client.write("test_row", "test_col", largetable_client::Record::new());
        batching_client.prepare_flush();
        batching_client.read("test_row", "test_col").unwrap();
    }

    fn mkrecord(input: &str) -> largetable_client::Record {
        let mut r = largetable_client::Record::new();
        r.set_data(Vec::from(input.as_bytes()));
        r
    }

    #[test]
    fn test_scoped_reads() {
        let c = largetable_test::LargeTableMockClient::new();
        let b = LargeTableBatchingClient::new_with_cache(c);
        b.write("food", "bread", mkrecord("bread"));
        b.write("food", "pizza", mkrecord("pizza"));
        b.write("food", "cookies", mkrecord("cookies"));
        b.write("food", "sandwich", mkrecord("sandwich"));
        b.write("food", "broccoli", mkrecord("broccoli"));
        b.write("dogs", "retriever", mkrecord("retriever"));
        b.write("dogs", "poodle", mkrecord("poodle"));
        b.write("dogs", "pomeranian", mkrecord("pomeranian"));
        b.write("dogs", "labrador", mkrecord("labrador"));

        assert_eq!(
            b.read_scoped("dogs", "p"),
            vec![mkrecord("pomeranian"), mkrecord("poodle")]
        );

        assert_eq!(
            b.read_scoped("food", ""),
            vec![
                mkrecord("bread"),
                mkrecord("broccoli"),
                mkrecord("cookies"),
                mkrecord("pizza"),
                mkrecord("sandwich")
            ]
        );

        assert_eq!(
            b.read_scoped("food", "b"),
            vec![mkrecord("bread"), mkrecord("broccoli"),]
        );
    }

    #[test]
    fn test_scoped_reads_with_flush() {
        let c = largetable_test::LargeTableMockClient::new();
        let b = LargeTableBatchingClient::new_with_cache(c);
        b.write("food", "bread", mkrecord("bread"));
        b.write("food", "pizza", mkrecord("pizza"));
        b.write("food", "cookies", mkrecord("cookies"));
        b.write("food", "sandwich", mkrecord("sandwich"));

        b.flush();

        b.write("food", "broccoli", mkrecord("broccoli"));
        b.write("dogs", "retriever", mkrecord("retriever"));
        b.write("dogs", "poodle", mkrecord("poodle"));
        b.write("dogs", "pomeranian", mkrecord("pomeranian"));
        b.write("dogs", "labrador", mkrecord("labrador"));

        assert_eq!(
            b.read_scoped("dogs", "p"),
            vec![mkrecord("pomeranian"), mkrecord("poodle")]
        );

        assert_eq!(
            b.read_scoped("food", ""),
            vec![
                mkrecord("bread"),
                mkrecord("broccoli"),
                mkrecord("cookies"),
                mkrecord("pizza"),
                mkrecord("sandwich")
            ]
        );

        assert_eq!(
            b.read_scoped("food", "b"),
            vec![mkrecord("bread"), mkrecord("broccoli"),]
        );
    }

    #[test]
    fn test_scoped_reads_with_flush_2() {
        let c = largetable_test::LargeTableMockClient::new();
        let b = LargeTableBatchingClient::new_with_cache(c);
        b.write("food", "bread", mkrecord("bread"));
        b.write("food", "pizza", mkrecord("pizza"));
        b.write("food", "cookies", mkrecord("cookies"));
        b.prepare_flush();
        b.write("food", "sandwich", mkrecord("sandwich"));
        b.write("food", "broccoli", mkrecord("broccoli"));
        b.perform_flush();
        b.write("dogs", "retriever", mkrecord("retriever"));
        b.write("dogs", "poodle", mkrecord("poodle"));
        b.finish_flush();
        b.write("dogs", "pomeranian", mkrecord("pomeranian"));
        b.write("dogs", "labrador", mkrecord("labrador"));

        assert_eq!(
            b.read_scoped("dogs", "p"),
            vec![mkrecord("pomeranian"), mkrecord("poodle")]
        );

        assert_eq!(
            b.read_scoped("food", ""),
            vec![
                mkrecord("bread"),
                mkrecord("broccoli"),
                mkrecord("cookies"),
                mkrecord("pizza"),
                mkrecord("sandwich")
            ]
        );

        assert_eq!(
            b.read_scoped("food", "b"),
            vec![mkrecord("bread"), mkrecord("broccoli"),]
        );
    }

    #[test]
    fn test_scoped_reads_with_flush_3() {
        let c = largetable_test::LargeTableMockClient::new();
        let b = LargeTableBatchingClient::new_with_cache(c);
        b.write("food", "bread", mkrecord("bread"));
        b.write("food", "pizza", mkrecord("pizza"));
        b.write("food", "cookies", mkrecord("cookies"));
        b.write("food", "sandwich", mkrecord("sandwich"));
        b.write("food", "broccoli", mkrecord("broccoli"));
        b.write("dogs", "retriever", mkrecord("retriever"));
        b.write("dogs", "poodle", mkrecord("poodle"));
        b.write("dogs", "pomeranian", mkrecord("pomeranian"));
        b.write("dogs", "labrador", mkrecord("labrador"));

        b.prepare_flush();
        assert_eq!(
            b.read_scoped("dogs", "p"),
            vec![mkrecord("pomeranian"), mkrecord("poodle")]
        );

        b.perform_flush();
        assert_eq!(
            b.read_scoped("food", ""),
            vec![
                mkrecord("bread"),
                mkrecord("broccoli"),
                mkrecord("cookies"),
                mkrecord("pizza"),
                mkrecord("sandwich")
            ]
        );

        b.finish_flush();
        assert_eq!(
            b.read_scoped("food", "b"),
            vec![mkrecord("bread"), mkrecord("broccoli"),]
        );
    }
}

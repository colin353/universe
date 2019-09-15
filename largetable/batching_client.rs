use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::RwLock;

struct BatchingClient<T, C: largetable_client::LargeTableClient> {
    cache: RwLock<VecDeque<BTreeMap<(String, String), T>>>,
    client: C,
}

impl<T: protobuf::Message + Clone, C: largetable_client::LargeTableClient> BatchingClient<T, C> {
    fn new(client: C) -> Self {
        Self {
            cache: RwLock::new(VecDeque::new()),
            client: client,
        }
    }

    fn new_with_cache(client: C) -> Self {
        Self {
            cache: RwLock::new(VecDeque::from(vec![BTreeMap::new()])),
            client: client,
        }
    }

    fn add_cache(&self) {
        self.cache.write().unwrap().push_back(BTreeMap::new());
    }

    fn read(&self, row: &str, col: &str) -> Option<T> {
        for cache in self.cache.read().unwrap().iter().rev() {
            if let Some(x) = cache.get(&(row.to_owned(), col.to_owned())) {
                return Some(x.clone());
            }
        }
        self.client.read_proto::<T>(row, col, 0)
    }

    fn flush(&mut self) {
        self.prepare_flush();
        self.perform_flush();
        self.finish_flush();
    }

    fn prepare_flush(&mut self) {
        self.cache.write().unwrap().push_back(BTreeMap::new());
    }

    fn finish_flush(&mut self) {
        self.cache.write().unwrap().pop_front();
    }

    fn perform_flush(&self) {
        let cache_locked = self.cache.read().unwrap();
        let cache = match cache_locked.front() {
            Some(x) => x,
            None => return,
        };

        let mut writer = largetable_client::LargeTableBatchWriter::new();
        for ((row, col), msg) in cache {
            writer.write_proto(row, col, 0, msg);
        }
        writer.finish(&self.client);
    }

    fn write(&self, row: &str, col: &str, message: T) {
        if let Some(cache) = self.cache.write().unwrap().back_mut() {
            cache.insert((row.to_owned(), col.to_owned()), message);
            return;
        }

        self.client.write_proto::<T>(row, col, 0, &message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

    #[test]
    fn test_write() {
        let client = largetable_test::LargeTableMockClient::new();
        let mut batching_client = BatchingClient::new(client);

        batching_client.write("test_row", "test_col", largetable_client::Record::new());
        batching_client.read("test_row", "test_col").unwrap();
    }

    #[test]
    fn test_write_with_cache() {
        let client = largetable_test::LargeTableMockClient::new();
        let mut batching_client = BatchingClient::new(client);
        batching_client.add_cache();
        batching_client.write("test_row", "test_col", largetable_client::Record::new());
        batching_client.read("test_row", "test_col").unwrap();
    }

    #[test]
    fn test_write_with_flush() {
        let client = largetable_test::LargeTableMockClient::new();
        let mut batching_client = BatchingClient::new(client);
        batching_client.add_cache();
        batching_client.write("test_row", "test_col", largetable_client::Record::new());
        batching_client.prepare_flush();
        batching_client.read("test_row", "test_col").unwrap();
    }
}

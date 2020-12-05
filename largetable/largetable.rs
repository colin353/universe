use dtable;
use itertools::{MinHeap, KV};
use keyserializer;
use largetable_proto_rust::Record;
use mtable;
use protobuf;
use recordio;
use sstable2::SSTableReader;

use std;
use std::borrow::BorrowMut;
use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::sync::RwLock;

pub struct LargeTable {
    mtables: Vec<RwLock<mtable::MTable>>,
    dtables: Vec<RwLock<dtable::DTable>>,
    journals: Vec<RwLock<recordio::RecordIOWriterOwned<Record>>>,
}

impl<'a> LargeTable {
    pub fn new() -> Self {
        LargeTable {
            mtables: vec![RwLock::new(mtable::MTable::new())],
            dtables: Vec::new(),
            journals: Vec::new(),
        }
    }

    pub fn add_journal(&mut self, writer: Box<io::Write + Send + Sync>) {
        self.journals
            .insert(0, RwLock::new(recordio::RecordIOWriter::new(writer)));
        self.journals.drain(1..);
    }

    pub fn add_mtable(&mut self) {
        self.mtables.insert(0, RwLock::new(mtable::MTable::new()));
    }

    pub fn drop_mtables(&mut self) {
        self.mtables.drain(1..);
    }

    pub fn load_from_journal(&mut self, reader: Box<io::Read>) {
        let records = recordio::RecordIOReaderOwned::<Record>::new(reader);
        for record in records {
            let row = record.get_row().to_owned();
            let col = record.get_col().to_owned();
            self.write(&row, &col, record);
        }
    }

    pub fn get_memory_usage(&self) -> u64 {
        self.mtables[0].read().unwrap().get_memory_usage()
    }

    // Try to read the record as a particular proto type.
    pub fn read_proto<T: protobuf::Message>(
        &self,
        row: &str,
        col: &str,
        timestamp: u64,
    ) -> io::Result<Option<T>> {
        // First, try to read the Record proto. If no result found, return None.
        let record = match self.read(row, col, timestamp) {
            Some(x) => x,
            None => return Ok(None),
        };

        record_to_proto(record)
    }

    pub fn reserve_id(&self, row: &str, col: &str) -> u64 {
        // First, take out write locks on the entire database. We need to make sure nobody else is
        // reserving a number first.
        let mut reserved_mtables = self
            .mtables
            .iter()
            .map(|m| m.write().unwrap())
            .collect::<Vec<_>>();
        let mut reserved_dtables = self
            .dtables
            .iter()
            .map(|m| m.write().unwrap())
            .collect::<Vec<_>>();

        let mut ids: Vec<u64> = Vec::new();
        for dt in reserved_dtables.iter_mut() {
            match dt.read(row, col, std::u64::MAX) {
                Some(x) => ids.push(x.get_timestamp()),
                None => continue,
            }
        }

        for mt in reserved_mtables.iter() {
            match mt.read(row, col, std::u64::MAX) {
                Some(x) => ids.push(x.get_timestamp()),
                None => continue,
            }
        }

        // We should select the record with the largest timestamp.
        let max_id = match ids.iter().max() {
            Some(x) => x.clone(),
            None => 0,
        };

        // Reserve the next index.
        let reserved_id = max_id + 1;

        // Prepare the record to write
        let mut record = Record::new();
        record.set_timestamp(reserved_id);

        // Then write to the journal, if necessary
        if self.journals.len() > 0 {
            record.set_col(col.to_owned());
            record.set_row(row.to_owned());
            self.journals[0].write().unwrap().write(&record);

            // The row and col should not be in the proto written to mtable, it just
            // wastes memory.
            record.clear_col();
            record.clear_row();
        }

        // Finally write the record
        reserved_mtables[0].write(row, col, record);

        reserved_id
    }

    pub fn read(&self, row: &str, col: &str, timestamp: u64) -> Option<Record> {
        let mut records: Vec<Record> = Vec::new();
        for dt in self.dtables.iter() {
            match dt.read().unwrap().read(row, col, timestamp) {
                Some(x) => {
                    records.push(x);
                }
                None => continue,
            }
        }

        for mt in self.mtables.iter() {
            match mt.read().unwrap().read(row, col, timestamp) {
                Some(x) => {
                    records.push(x.clone());
                }
                None => continue,
            }
        }

        // We should select the record with the largest timestamp.
        let mut max_timestamp = 0;
        let mut record_index = 0;
        let mut found = false;
        for (i, r) in records.iter().enumerate() {
            if r.get_timestamp() >= max_timestamp && r.get_timestamp() <= timestamp {
                found = true;
                record_index = i;
                max_timestamp = r.get_timestamp();
            }
        }

        match found {
            true => Some(records[record_index].clone()),
            false => None,
        }
    }

    pub fn read_range(
        &self,
        row: &str,
        col_spec: &str,
        min_col: &str,
        max_col: &str,
        limit: u64,
        timestamp: u64,
    ) -> Vec<Record> {
        let mut data: Vec<VecDeque<Record>> = Vec::new();
        let mut output: Vec<Record> = Vec::new();

        // Get records from each mtable.
        for mt in self.mtables.iter() {
            data.push(VecDeque::from(
                mt.read()
                    .unwrap()
                    .read_range(row, col_spec, min_col, max_col, limit, timestamp),
            ));
        }

        // Get records from each dtable.
        for dt in self.dtables.iter() {
            data.push(VecDeque::from(
                dt.read()
                    .unwrap()
                    .read_range(row, col_spec, min_col, max_col, limit, timestamp),
            ))
        }

        // Merge the records together. First, fill the BTreeMap with
        // a record from each dataset.
        let mut heap = MinHeap::<KV<String, (usize, Record)>>::new();
        for i in 0..data.len() {
            match data[i].pop_front() {
                Some(x) => heap.push(KV::new(keyserializer::key_from_record(&x), (i, x))),
                None => continue,
            };
        }

        // Now keep popping off records in order, and refreshing it with the
        // corresponding list
        let mut prev_col = String::new();
        let mut prev_record = Record::new();
        let mut found_record = false;
        loop {
            let (key, index, record) = {
                let kv = match heap.pop() {
                    Some(x) => x,
                    None => break,
                };
                let KV(key, value) = kv;
                (key, value.0, value.1)
            };

            // If the record is a deletion, omit it from the read_range.
            if found_record && record.get_col() != prev_col && !prev_record.get_deleted() {
                output.push(prev_record);

                // If we already have enough values to pass the limit,
                // just quit.
                if output.len() >= (limit as usize) {
                    return output;
                }
            }
            found_record = true;
            prev_col = record.get_col().to_owned();
            prev_record = record;

            // Pop off the record from the list it came from.
            match data[index].pop_front() {
                Some(x) => heap.push(KV::new(keyserializer::key_from_record(&x), (index, x))),
                None => continue,
            };
        }

        // Push the last one in there as well, unless it was a deletion.
        if found_record && !prev_record.get_deleted() {
            output.push(prev_record);
        }

        output
    }

    pub fn get_shard_hint(
        &self,
        row: &str,
        col_spec: &str,
        min_col: &str,
        max_col: &str,
    ) -> Vec<String> {
        let key_spec = keyserializer::get_colspec(row, col_spec);
        let min_key = keyserializer::get_colspec(row, min_col);
        let max_key = if max_col == "" {
            String::from("")
        } else {
            keyserializer::get_colspec(&row, &min_key)
        };

        let mut shards = vec![];

        for dt in self.dtables.iter() {
            shards.extend_from_slice(
                dt.read()
                    .unwrap()
                    .get_shard_hint(&key_spec, &min_key, &max_key)
                    .as_slice(),
            );
        }

        // TODO: make this more efficient. Each get_shard_hint call gives a sorted list, so we can
        // just merge them rather than doing a whole sort at the end.
        shards.sort_unstable();
        shards
    }

    pub fn write_to_disk(&self, w: &mut io::Write, idx: usize) {
        let write: &mut io::Write = w.borrow_mut() as &mut io::Write;
        self.mtables[idx]
            .write()
            .unwrap()
            .write_to_disk(write)
            .unwrap();
    }

    pub fn add_dtable_from_sstable(&mut self, reader: SSTableReader<Record>) {
        self.dtables
            .push(RwLock::new(dtable::DTable::from_sstable(reader)));
    }

    pub fn add_dtable(&mut self, reader: std::fs::File) {
        self.dtables
            .push(RwLock::new(dtable::DTable::new(reader).unwrap()));
    }

    pub fn clear_dtables(&mut self) {
        self.dtables = Vec::new();
    }

    pub fn write(&self, row: &str, col: &str, mut record: Record) {
        // By tradition, just write to the first entry in the mtable vector.
        assert!(self.mtables.len() > 0);

        // Then write to the journal, if necessary.
        if self.journals.len() > 0 {
            record.set_col(col.to_owned());
            record.set_row(row.to_owned());
            self.journals[0].write().unwrap().write(&record);

            // The row and col should not be in the proto written to mtable, it just
            // wastes memory.
            record.clear_col();
            record.clear_row();
        }

        // Finally, write to memory.
        self.mtables[0].write().unwrap().write(row, col, record);
    }

    // Write an arbitary proto by converting the proto to a Record.
    pub fn write_proto<T: protobuf::Message>(
        &self,
        row: &str,
        col: &str,
        timestamp: u64,
        message: &T,
    ) {
        let record = proto_to_record(row.to_owned(), col.to_owned(), timestamp, message);
        self.write(row, col, record)
    }
}

fn record_to_proto<T: protobuf::Message>(record: Record) -> io::Result<Option<T>> {
    // If the Record was deleted, return None.
    if record.get_deleted() {
        return Ok(None);
    }

    // Now try to decode that Record into the specified proto.
    let mut message = T::new();
    match message.merge_from_bytes(record.get_data()) {
        Ok(_) => Ok(Some(message)),
        Err(_) => Err(::std::io::Error::new(
            ::std::io::ErrorKind::InvalidData,
            "Unable to deserialize protobuf",
        )),
    }
}

fn proto_to_record<T: protobuf::Message>(
    row: String,
    col: String,
    timestamp: u64,
    message: &T,
) -> Record {
    let mut record = Record::new();
    record.set_timestamp(timestamp);
    record.set_row(row);
    record.set_col(col);

    let mut serialized_proto = Vec::new();
    {
        let mut c = ::std::io::Cursor::new(&mut serialized_proto);
        message
            .write_to_writer(&mut c)
            .expect("Failed to serialize protobuf!");
    }

    record.set_data(serialized_proto);
    return record;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std;

    fn record(data: Vec<u8>, timestamp: u64, deleted: bool) -> Record {
        let mut r = Record::new();
        r.set_timestamp(timestamp);
        r.set_data(data);
        r.set_deleted(deleted);
        r
    }

    #[test]
    fn read_and_write_mtable() {
        let l = LargeTable::new();
        l.write("asdf", "fdsa", record(vec![12, 23, 34], 123, false));
        l.write("test", "test", record(vec![99], 555, true));
        l.write("asdf", "fdsa", record(vec![42], 234, false));

        assert_eq!(l.read("asdf", "fdsa", 999).unwrap().get_data(), &[42]);
    }

    #[test]
    fn read_and_write_mtable_and_dtable() {
        let mut l = LargeTable::new();

        l.write("test", "test", record(vec![13], 123, false));
        l.write("test", "test", record(vec![42], 234, false));

        assert_eq!(l.read("test", "test", 500).unwrap().get_data(), &[42]);

        let mut f = std::io::Cursor::new(Vec::new());
        l.write_to_disk(&mut f, 0);
        l.add_mtable();
        l.drop_mtables();

        // Writing to disk should clear the mtable, so we shouldn't get
        // any results.
        assert_eq!(l.read("test", "test", 500), None);

        l.add_dtable_from_sstable(SSTableReader::from_bytes(&f.into_inner()).unwrap());

        // Now we should be reading from the dtable, not the mtable.
        assert_eq!(l.read("test", "test", 500).unwrap().get_data(), &[42]);

        // And we should still be able to write to the mtable.
        l.write("test", "test", record(vec![99], 400, false));
        assert_eq!(l.read("test", "test", 500).unwrap().get_data(), &[99]);
    }

    fn make_record(row: &str, col: &str, timestamp: u64) -> Record {
        let mut r = Record::new();
        r.set_row(row.to_owned());
        r.set_col(col.to_owned());
        r.set_timestamp(timestamp);
        r
    }

    fn write_record(t: &mut LargeTable, row: &str, col: &str, timestamp: u64) {
        t.write(row, col, make_record(row, col, timestamp));
    }

    #[test]
    fn read_range_dtable() {
        let mut l = LargeTable::new();
        write_record(&mut l, "a", "apple", 1234);
        write_record(&mut l, "a", "cantaloupe", 1234);
        write_record(&mut l, "a", "cherry", 1234);
        write_record(&mut l, "a", "durian", 1234);
        write_record(&mut l, "a", "fruit", 1234);

        // Write this to a DTable.
        let mut f = std::io::Cursor::new(Vec::new());
        l.write_to_disk(&mut f, 0);
        l.add_dtable_from_sstable(SSTableReader::from_bytes(&f.into_inner()).unwrap());

        write_record(&mut l, "a", "avocado", 1234);
        write_record(&mut l, "a", "couscous", 1234);
        write_record(&mut l, "a", "corn", 1234);
        write_record(&mut l, "a", "dandelion", 1234);
        write_record(&mut l, "a", "fig", 1234);

        // Write this to a DTable.
        let mut f = std::io::Cursor::new(Vec::new());
        l.write_to_disk(&mut f, 0);
        l.add_dtable_from_sstable(SSTableReader::from_bytes(&f.into_inner()).unwrap());

        // Read it out again.
        let out = l.read_range("a", "c", "", "", 100, 1234);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], make_record("a", "cantaloupe", 1234));
        assert_eq!(out[1], make_record("a", "cherry", 1234));
        assert_eq!(out[2], make_record("a", "corn", 1234));
        assert_eq!(out[3], make_record("a", "couscous", 1234));
    }

    #[test]
    fn read_range_mtable() {
        let mut l = LargeTable::new();
        write_record(&mut l, "a", "apple", 1234);
        write_record(&mut l, "a", "cantaloupe", 1234);
        write_record(&mut l, "a", "cherry", 1234);
        write_record(&mut l, "a", "durian", 1234);
        write_record(&mut l, "a", "fruit", 1234);

        write_record(&mut l, "a", "avocado", 1234);
        write_record(&mut l, "a", "couscous", 1234);
        write_record(&mut l, "a", "corn", 1234);
        write_record(&mut l, "a", "dandelion", 1234);
        write_record(&mut l, "a", "fig", 1234);

        // Read it out again.
        let out = l.read_range("a", "c", "", "", 100, 1234);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], make_record("a", "cantaloupe", 1234));
        assert_eq!(out[1], make_record("a", "cherry", 1234));
        assert_eq!(out[2], make_record("a", "corn", 1234));
        assert_eq!(out[3], make_record("a", "couscous", 1234));
    }

    #[test]
    fn read_range_with_multiple_rows_mtable() {
        let mut l = LargeTable::new();
        write_record(&mut l, "a", "apple", 1234);
        write_record(&mut l, "a", "cantaloupe", 1234);
        write_record(&mut l, "a", "cherry", 1234);
        write_record(&mut l, "a", "durian", 1234);
        write_record(&mut l, "a", "fruit", 1234);

        write_record(&mut l, "b", "avocado", 1234);
        write_record(&mut l, "b", "couscous", 1234);
        write_record(&mut l, "b", "corn", 1234);
        write_record(&mut l, "b", "dandelion", 1234);
        write_record(&mut l, "b", "fig", 1234);

        // Read it out again.
        let out = l.read_range("a", "c", "", "", 100, 1234);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], make_record("a", "cantaloupe", 1234));
        assert_eq!(out[1], make_record("a", "cherry", 1234));
    }

    #[test]
    fn read_range_with_timestamp() {
        let mut l = LargeTable::new();
        write_record(&mut l, "a", "apple", 1000);
        write_record(&mut l, "a", "cantaloupe", 3000);
        write_record(&mut l, "a", "cherry", 1000);
        write_record(&mut l, "a", "durian", 3000);
        write_record(&mut l, "a", "fruit", 1000);

        // Write this to a DTable.
        let mut f = std::io::Cursor::new(Vec::new());
        l.write_to_disk(&mut f, 0);
        l.add_dtable_from_sstable(SSTableReader::from_bytes(&f.into_inner()).unwrap());

        write_record(&mut l, "a", "avocado", 3000);
        write_record(&mut l, "a", "couscous", 1000);
        write_record(&mut l, "a", "corn", 3000);
        write_record(&mut l, "a", "dandelion", 1000);
        write_record(&mut l, "a", "fig", 3000);

        // Write this to a DTable.
        let mut f = std::io::Cursor::new(Vec::new());
        l.write_to_disk(&mut f, 0);
        l.add_dtable_from_sstable(SSTableReader::from_bytes(&f.into_inner()).unwrap());

        // Read it out again.
        let out = l.read_range("a", "", "", "", 100, 1234);
        assert_eq!(out.len(), 5);
        assert_eq!(out[0], make_record("a", "apple", 1000));
        assert_eq!(out[1], make_record("a", "cherry", 1000));
        assert_eq!(out[2], make_record("a", "couscous", 1000));
        assert_eq!(out[3], make_record("a", "dandelion", 1000));
        assert_eq!(out[4], make_record("a", "fruit", 1000));
    }

    #[test]
    fn read_range_with_deletion() {
        let mut l = LargeTable::new();
        write_record(&mut l, "a", "apple", 1000);
        write_record(&mut l, "a", "cantaloupe", 3000);
        write_record(&mut l, "a", "cherry", 1000);
        write_record(&mut l, "a", "durian", 3000);
        write_record(&mut l, "a", "fruit", 1000);

        // Read it out again.
        let out = l.read_range("a", "", "", "", 100, 9999);
        assert_eq!(out.len(), 5);
        assert_eq!(out[0], make_record("a", "apple", 1000));
        assert_eq!(out[1], make_record("a", "cantaloupe", 3000));
        assert_eq!(out[2], make_record("a", "cherry", 1000));
        assert_eq!(out[3], make_record("a", "durian", 3000));
        assert_eq!(out[4], make_record("a", "fruit", 1000));

        // Delete one of the records
        let mut record = make_record("a", "cantaloupe", 4000);
        record.set_deleted(true);
        l.write("a", "cantaloupe", record);

        // Read it out again.
        let out = l.read_range("a", "", "", "", 100, 9999);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], make_record("a", "apple", 1000));
        assert_eq!(out[1], make_record("a", "cherry", 1000));
        assert_eq!(out[2], make_record("a", "durian", 3000));
        assert_eq!(out[3], make_record("a", "fruit", 1000));
    }

    #[test]
    fn read_range_with_timestamp_on_mtable() {
        let mut l = LargeTable::new();
        write_record(&mut l, "a", "apple", 1000);
        write_record(&mut l, "a", "cantaloupe", 3000);
        write_record(&mut l, "a", "cherry", 1000);
        write_record(&mut l, "a", "durian", 3000);
        write_record(&mut l, "a", "fruit", 1000);

        write_record(&mut l, "a", "avocado", 3000);
        write_record(&mut l, "a", "couscous", 1000);
        write_record(&mut l, "a", "corn", 3000);
        write_record(&mut l, "a", "dandelion", 1000);
        write_record(&mut l, "a", "fig", 3000);

        // Read it out again.
        let out = l.read_range("a", "", "", "", 100, 1234);
        assert_eq!(out.len(), 5);
        assert_eq!(out[0], make_record("a", "apple", 1000));
        assert_eq!(out[1], make_record("a", "cherry", 1000));
        assert_eq!(out[2], make_record("a", "couscous", 1000));
        assert_eq!(out[3], make_record("a", "dandelion", 1000));
        assert_eq!(out[4], make_record("a", "fruit", 1000));
    }

    #[test]
    fn test_reserve_id() {
        let l = LargeTable::new();
        assert_eq!(l.reserve_id("hello", "world"), 1);
        assert_eq!(l.reserve_id("hello", "world"), 2);
        assert_eq!(l.reserve_id("hello", "zerld"), 1);
        assert_eq!(l.reserve_id("hello", "world"), 3);
    }

    #[test]
    fn read_and_write_proto() {
        let l = LargeTable::new();

        let record = make_record("fake_serialized", "abcdef", 12345);
        l.write_proto("row", "col", 55555, &record);

        let result = l
            .read_proto::<Record>("row", "col", 99999)
            .unwrap()
            .unwrap();

        assert_eq!(result.get_row(), "fake_serialized");
        assert_eq!(result.get_col(), "abcdef");
        assert_eq!(result.get_timestamp(), 12345);
    }

    #[test]
    fn identical_records_in_dtables() {
        let mut l = LargeTable::new();
        write_record(&mut l, "a", "apple", 1000);
        write_record(&mut l, "a", "alternate", 1000);

        // Write this to a DTable.
        let mut f = std::io::Cursor::new(Vec::new());
        l.write_to_disk(&mut f, 0);
        l.add_dtable_from_sstable(SSTableReader::from_bytes(&f.into_inner()).unwrap());

        write_record(&mut l, "a", "apple", 1000);
        write_record(&mut l, "a", "secret", 1000);

        // Read it out again.
        let out = l.read_range("a", "", "", "", 100, 1234);
        //assert_eq!(out.len(), 3);
        assert_eq!(out[0], make_record("a", "alternate", 1000));
        assert_eq!(out[1], make_record("a", "apple", 1000));
        assert_eq!(out[2], make_record("a", "secret", 1000));
    }
}

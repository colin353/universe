/*
 * mtable.rs
 *
 * This code defines the MTable struct, which is the memory table. Because SSTables (and thus
 * DTables) are immutable, we need a mutable in-memory data structure to store updates before
 * they are frozen to disk. That's what the MTable is for.
 */

use keyserializer;
use largetable_proto_rust::Record;

use sstable;

use std;
use std::collections::BTreeMap;
use std::collections::Bound;
use std::io;

pub struct MTable {
    tree: BTreeMap<String, Record>,
    memory_usage: u64,
}

impl<'short, 'long: 'short> MTable {
    pub fn new() -> MTable {
        MTable {
            tree: BTreeMap::new(),
            memory_usage: 0,
        }
    }

    pub fn get_memory_usage(&self) -> u64 {
        self.memory_usage
    }

    pub fn write(&mut self, row: &str, col: &str, mut record: Record) {
        self.memory_usage += record.data.len() as u64;
        record.set_row(row.to_string());
        record.set_col(col.to_string());
        self.tree.insert(
            keyserializer::serialize_key(row, col, record.get_timestamp()),
            record,
        );
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
        let min_key = if min_col == "" {
            keyserializer::get_colspec(row, col_spec)
        } else {
            keyserializer::get_colspec(row, min_col)
        };

        let iter = if max_col == "" {
            self.tree
                .range((Bound::Included(min_key), Bound::Unbounded))
        } else {
            self.tree.range((
                Bound::Included(min_key),
                Bound::Excluded(keyserializer::get_colspec(row, max_col)),
            ))
        };

        let mut output = Vec::new();
        let mut found = false;
        let mut current_value = Record::new();
        for (_, value) in iter {
            if value.get_row() != row {
                break;
            }

            if !value.get_col().starts_with(col_spec) {
                break;
            }

            if value.get_timestamp() > timestamp {
                continue;
            }

            if found && value.get_col() != current_value.get_col() {
                output.push(current_value);

                // Quit if we already reached the size limit
                if output.len() == (limit as usize) {
                    return output;
                }
            }

            found = true;
            current_value = value.clone();
        }

        if found {
            output.push(current_value);
        }

        output
    }

    pub fn read(&'long self, row: &str, col: &str, timestamp: u64) -> Option<&'short Record> {
        let key_spec = keyserializer::get_keyspec(row, col);
        let iterator = self
            .tree
            .range((Bound::Excluded(key_spec.to_owned()), Bound::Unbounded));

        let mut target_key = String::new();
        let mut found = false;
        for (key, value) in iterator {
            if !key.starts_with(key_spec.as_str()) {
                break;
            }
            if value.get_timestamp() > timestamp {
                break;
            }

            found = true;
            target_key = key.to_owned();
        }

        if !found {
            return None;
        }

        self.tree.get(target_key.as_str())
    }

    pub fn write_to_disk(&self, write: &mut io::Write) -> io::Result<()> {
        let mut table = sstable::SSTableBuilder::new(write);
        for (key, value) in self.tree.iter() {
            table.write_ordered(key, value.clone())?;
        }
        table.finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dtable;
    use std;
    use std::io::Seek;
    #[test]
    fn read_and_write_mtable() {
        let mut memtable = MTable::new();
        let mut r = Record::new();
        r.set_data(vec![12, 23, 34, 45]);
        r.set_timestamp(1234);
        memtable.write("row", "column", r);
        assert_eq!(
            memtable.read("row", "column", 2345).unwrap().get_data(),
            &[12, 23, 34, 45]
        );
    }

    #[test]
    fn read_and_write_dtable_with_timestamp() {
        let mut memtable = MTable::new();
        let mut r = Record::new();
        r.set_data(vec![12, 23, 34, 45]);
        r.set_timestamp(1234);
        memtable.write("row", "column", r);

        let mut r = Record::new();
        r.set_data(vec![11, 22, 33, 44]);
        r.set_timestamp(1234);
        memtable.write("weird", "thing", r);

        let mut r = Record::new();
        r.set_timestamp(2345);
        r.set_data(vec![99]);
        memtable.write("row", "column", r);

        assert_eq!(
            memtable.read("row", "column", 1500).unwrap().get_data(),
            &[12, 23, 34, 45]
        );
    }

    #[test]
    fn read_data_before_timestamp() {
        let mut memtable = MTable::new();
        let mut r = Record::new();
        r.set_data(vec![12, 23, 34, 45]);
        r.set_timestamp(1234);
        memtable.write("row", "column", r);

        let mut r = Record::new();
        r.set_timestamp(2345);
        r.set_data(vec![99]);
        memtable.write("row", "column", r);

        assert_eq!(memtable.read("row", "column", 5), None);
    }

    #[test]
    fn read_nonexistent_data() {
        let mut memtable = MTable::new();
        let mut r = Record::new();
        r.set_data(vec![12, 23, 34, 45]);
        r.set_timestamp(1234);
        memtable.write("row", "column", r);

        let mut r = Record::new();
        r.set_timestamp(2345);
        r.set_data(vec![99]);
        memtable.write("row", "column", r);

        assert_eq!(memtable.read("rowe", "columne", 9999), None);
    }

    #[test]
    fn write_mtable_as_dtable() {
        let mut memtable = MTable::new();
        let mut r = Record::new();
        r.set_data(vec![12, 23, 34, 45]);
        r.set_timestamp(1234);
        memtable.write("row", "column", r);

        let mut r = Record::new();
        r.set_timestamp(2345);
        r.set_data(vec![99]);
        memtable.write("row", "column", r);

        let mut d = std::io::Cursor::new(Vec::new());
        memtable.write_to_disk(&mut d).unwrap();

        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut disktable = dtable::DTable::new(Box::new(d)).unwrap();

        assert_eq!(
            disktable.read("row", "column", 9999).unwrap().get_data(),
            &[99]
        );

        assert_eq!(disktable.read("row", "column", 0), None);
    }

    fn add_record(m: &mut MTable, row: &str, col: &str, timestamp: u64) {
        let mut r = Record::new();
        r.set_data(vec![12, 23, 34, 45]);
        r.set_timestamp(timestamp);
        m.write(row, col, r);
    }

    fn record(row: &str, col: &str, timestamp: u64) -> Record {
        let mut r = Record::new();
        r.set_timestamp(timestamp);
        r.set_data(vec![12, 23, 34, 45]);
        r.set_row(row.to_owned());
        r.set_col(col.to_owned());
        r
    }

    #[test]
    fn read_range_with_spec() {
        let mut m = MTable::new();
        add_record(&mut m, "people", "adam", 1234);
        add_record(&mut m, "people", "calhoun", 1234);
        add_record(&mut m, "people", "colin", 1234);
        add_record(&mut m, "people", "daniel", 1234);
        add_record(&mut m, "people", "elvis", 1234);

        let output = m.read_range("people", "c", "", "", 100, 3000);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], record("people", "calhoun", 1234));
        assert_eq!(output[1], record("people", "colin", 1234));
    }

    #[test]
    fn read_range_with_min_max() {
        let mut m = MTable::new();
        add_record(&mut m, "people", "adam", 1234);
        add_record(&mut m, "people", "calhoun", 1234);
        add_record(&mut m, "people", "colin", 1234);
        add_record(&mut m, "people", "daniel", 1234);
        add_record(&mut m, "people", "elvis", 1234);

        let output = m.read_range("people", "", "colin", "daniel_", 100, 3000);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], record("people", "colin", 1234));
        assert_eq!(output[1], record("people", "daniel", 1234));
    }

    #[test]
    fn read_range_with_multiple_timestmap() {
        let mut m = MTable::new();
        add_record(&mut m, "people", "adam", 1234);
        // This record shouldn't appear, since there's a later one
        // which replaces it.
        add_record(&mut m, "people", "calhoun", 1234);
        add_record(&mut m, "people", "calhoun", 2000);
        // This record shouldn't appear because it comes after the
        // query timestamp.
        add_record(&mut m, "people", "calhoun", 4000);
        add_record(&mut m, "people", "colin", 1234);

        let output = m.read_range("people", "", "", "", 100, 3000);
        assert_eq!(output.len(), 3);
        assert_eq!(output[0], record("people", "adam", 1234));
        assert_eq!(output[1], record("people", "calhoun", 2000));
        assert_eq!(output[2], record("people", "colin", 1234));
    }
}

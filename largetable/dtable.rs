/*
 * dtable.rs
 *
 * This code defines the DTable struct, which is basically an sstable.
 */

use keyserializer;
use largetable_proto_rust::Record;
use sstable::{SSTableBuilder, SSTableReader, SpecdSSTableReader};

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct DTable {
    table: SSTableReader<Record>,
}

impl DTable {
    pub fn new(f: std::fs::File) -> io::Result<DTable> {
        Ok(DTable {
            table: SSTableReader::new(f)?,
        })
    }

    pub fn from_sstable(s: SSTableReader<Record>) -> Self {
        DTable { table: s }
    }

    pub fn from_bytes(b: &[u8]) -> io::Result<DTable> {
        Ok(DTable {
            table: SSTableReader::from_bytes(b)?,
        })
    }

    pub fn read(&self, row: &str, col: &str, timestamp: u64) -> Option<Record> {
        let key_spec = keyserializer::get_keyspec(row, col);

        let specd_reader = SpecdSSTableReader::from_reader(&self.table, key_spec.as_str());
        let mut target_value = Record::new();
        let mut found = false;
        for (_, value) in specd_reader {
            if value.get_timestamp() > timestamp {
                break;
            }

            found = true;
            target_value = value;
        }

        if !found {
            return None;
        }

        Some(target_value)
    }

    pub fn get_shard_hint(&self, key_spec: &str, min_key: &str, max_key: &str) -> Vec<String> {
        // The SSTable gives shard hints with string keys, but we want to return column keys. So
        // we'll remap the string keys into column keys.
        self.table
            .suggest_shards(key_spec, min_key, max_key)
            .iter()
            .map(|s| keyserializer::deserialize_col(s).to_owned())
            .collect()
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
        let key_spec = keyserializer::get_colspec(row, col_spec);
        let min_key = keyserializer::get_colspec(row, min_col);
        let max_key = if max_col != "" {
            keyserializer::get_colspec(row, max_col)
        } else {
            String::from("")
        };

        let specd_reader = SpecdSSTableReader::from_reader_with_scope(
            &self.table,
            key_spec.as_str(),
            min_key.as_str(),
            max_key.as_str(),
        );
        let mut found = false;
        let mut output = Vec::new();
        let mut current_key = String::from("");
        let mut current_value = Record::new();
        for (key, value) in specd_reader {
            // Ignore all records after our desired timestamp.
            if value.get_timestamp() > timestamp {
                continue;
            }

            // Check if we are looking at different versions of the same record,
            // or we have moved onto a new record.
            let col = keyserializer::deserialize_col(key.as_str());
            if current_key != "" && col != current_key {
                output.push(current_value);
                // If we already reached the specified size limit, quit.
                if output.len() == (limit as usize) {
                    return output;
                }
            }

            found = true;
            current_key = col.to_owned();
            current_value = value;
        }

        if found {
            output.push(current_value);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std;
    use std::io::Seek;
    #[test]
    fn read_and_write_dtable() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Record, _>::new(&mut d);
            let mut r = Record::new();
            r.set_timestamp(1234);
            r.set_data(vec![12, 23, 34, 45]);
            t.write_ordered(
                keyserializer::serialize_key("row", "column", 1234).as_str(),
                r,
            )
            .unwrap();

            let mut r = Record::new();
            r.set_timestamp(2345);
            r.set_data(vec![99]);
            t.write_ordered(
                keyserializer::serialize_key("row", "column", 2345).as_str(),
                r,
            )
            .unwrap();

            t.finish().unwrap();
        }
        let mut dt = DTable::from_sstable(SSTableReader::from_bytes(&d.into_inner()).unwrap());
        assert_eq!(dt.read("row", "column", 5000).unwrap().get_data(), &[99]);
        assert_eq!(dt.read("non-value", "non-column", 5000), None);
    }

    fn add_record<W: std::io::Write>(
        sstable: &mut SSTableBuilder<Record, W>,
        row: &str,
        col: &str,
        timestamp: u64,
    ) {
        sstable
            .write_ordered(
                keyserializer::serialize_key(row, col, timestamp).as_str(),
                record(row, col, timestamp),
            )
            .unwrap();
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
    fn read_specd() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Record, _>::new(&mut d);
            add_record(&mut t, "people", "calhoun", 1234);
            add_record(&mut t, "people", "colin", 1234);
            add_record(&mut t, "people", "daniel", 1234);
            add_record(&mut t, "people", "elvis", 1234);
            t.finish().unwrap();
        }
        let mut dt = DTable::from_sstable(SSTableReader::from_bytes(&d.into_inner()).unwrap());
        let output = dt.read_range("people", "", "", "", 100, 3000);
        assert_eq!(output.len(), 4);
        assert_eq!(output[0], record("people", "calhoun", 1234));
        assert_eq!(output[1], record("people", "colin", 1234));
        assert_eq!(output[2], record("people", "daniel", 1234));
        assert_eq!(output[3], record("people", "elvis", 1234));
    }

    #[test]
    fn read_specd_2() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Record, _>::new(&mut d);
            add_record(&mut t, "people", "adam", 1234);
            add_record(&mut t, "people", "calhoun", 1234);
            add_record(&mut t, "people", "colin", 1234);
            add_record(&mut t, "people", "daniel", 1234);
            add_record(&mut t, "people", "elvis", 1234);
            t.finish().unwrap();
        }
        let mut dt = DTable::from_sstable(SSTableReader::from_bytes(&d.into_inner()).unwrap());
        let output = dt.read_range("people", "c", "", "", 100, 3000);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], record("people", "calhoun", 1234));
        assert_eq!(output[1], record("people", "colin", 1234));
    }

    #[test]
    fn read_specd_3() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Record, _>::new(&mut d);
            add_record(&mut t, "people", "adam", 1234);
            add_record(&mut t, "people", "calhoun", 1234);
            add_record(&mut t, "people", "colin", 1234);
            add_record(&mut t, "people", "daniel", 1234);
            add_record(&mut t, "people", "elvis", 1234);
            t.finish().unwrap();
        }
        let mut dt = DTable::from_sstable(SSTableReader::from_bytes(&d.into_inner()).unwrap());
        let output = dt.read_range("people", "", "colin", "daniel_", 100, 3000);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], record("people", "colin", 1234));
        assert_eq!(output[1], record("people", "daniel", 1234));
    }

    #[test]
    fn read_specd_with_multiple_timestamp() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Record, _>::new(&mut d);
            add_record(&mut t, "people", "adam", 1234);
            // This record shouldn't appear, since there's a later one
            // which replaces it.
            add_record(&mut t, "people", "calhoun", 1234);
            add_record(&mut t, "people", "calhoun", 2000);
            // This record shouldn't appear because it comes after the
            // query timestamp.
            add_record(&mut t, "people", "calhoun", 4000);
            add_record(&mut t, "people", "colin", 1234);
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        let bytes = d.into_inner();
        {
            let mut dt = DTable::from_sstable(SSTableReader::from_bytes(&bytes).unwrap());
            let output = dt.read_range("people", "", "", "", 100, 3000);
            assert_eq!(output.len(), 3);
            assert_eq!(output[0], record("people", "adam", 1234));
            assert_eq!(output[1], record("people", "calhoun", 2000));
            assert_eq!(output[2], record("people", "colin", 1234));
        }
    }
}

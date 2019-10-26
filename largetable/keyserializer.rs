/*
 * keyserializer.rs
 *
 * This code defines the key serialization strategy for all components of the largetable
 * system.
 */

extern crate largetable_proto_rust;
use largetable_proto_rust::Record;

// serialize_key generates a key string based upon the row, column, and timestamp.
pub fn serialize_key(row: &str, col: &str, timestamp: u64) -> String {
    format!("{}\x00{}\x00{:016x}", row, col, timestamp)
}

pub fn key_from_record(record: &Record) -> String {
    serialize_key(record.get_row(), record.get_col(), record.get_timestamp())
}

// get_keyspec creates the first part of the key (the key spec) for a given row and
// column.
pub fn get_keyspec(row: &str, col: &str) -> String {
    format!("{}\x00{}\x00", row, col)
}

pub fn get_colspec(row: &str, col: &str) -> String {
    format!("{}\x00{}", row, col)
}

pub fn deserialize_key(key: &str) -> (&str, &str) {
    let split: Vec<&str> = key.split("\x00").collect();
    (split[0], split[1])
}

pub fn get_prefix(key: &str) -> &str {
    match key.rfind("\x00") {
        Some(idx) => &key[0..idx],
        None => "",
    }
}

pub fn deserialize_col(key: &str) -> &str {
    let split: Vec<&str> = key.split("\x00").collect();
    split[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_keyspec() {
        assert_eq!(
            serialize_key("hello", "world", 1024),
            "hello\x00world\x000000000000000400"
        );
    }
}

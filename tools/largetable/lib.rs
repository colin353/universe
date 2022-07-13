mod dtable;
mod mtable;

use bus::{DeserializeOwned, Serialize};

use std::sync::RwLock;

pub struct LargeTable<'a, W: std::io::Write> {
    mtables: Vec<RwLock<mtable::MTable>>,
    dtables: Vec<RwLock<dtable::DTable<'a>>>,
    journals: Vec<RwLock<recordio::RecordIOBuilder<internals::JournalEntry, W>>>,
}

impl<'a, W: std::io::Write> LargeTable<'a, W> {
    pub fn new() -> Self {
        Self {
            mtables: Vec::new(),
            dtables: Vec::new(),
            journals: Vec::new(),
        }
    }

    pub fn add_journal(&mut self, writer: W) {
        self.journals.clear();
        self.journals
            .insert(0, RwLock::new(recordio::RecordIOBuilder::new(writer)));
    }

    pub fn add_mtable(&mut self) {
        self.mtables.insert(0, RwLock::new(mtable::MTable::new()));
    }

    #[cfg(test)]
    pub fn add_dtable_from_bytes(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.dtables
            .insert(0, RwLock::new(dtable::DTable::from_bytes(bytes)?));
        Ok(())
    }

    pub fn load_from_journal<R: std::io::Read>(&mut self, reader: R) -> std::io::Result<()> {
        let mut journal = recordio::RecordIOReader::<internals::JournalEntry, _>::new(reader);
        while let Some(entry) = journal.next() {
            let entry = entry?;
            self.write_record(entry.row, entry.column, entry.record.as_view())?;
        }
        Ok(())
    }

    pub fn write_record(
        &self,
        row: String,
        column: String,
        record: internals::RecordView,
    ) -> std::io::Result<()> {
        if self.mtables.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "there are no mtables to write to!",
            ));
        }

        if row.contains("\x00") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "row names cannot contain the null byte!",
            ));
        }

        let entry = internals::JournalEntry {
            record: record.to_owned()?,
            row,
            column,
        };

        if self.journals.len() > 0 {
            self.journals[0]
                .write()
                .expect("failed to acquire write lock")
                .write(&entry)?;
        }

        self.mtables[0]
            .write()
            .expect("failed to acquire write lock")
            .write(entry.row, entry.column, entry.record);
        Ok(())
    }

    pub fn write<T: Serialize>(
        &self,
        row: String,
        column: String,
        timestamp: u64,
        message: T,
    ) -> std::io::Result<()> {
        let mut record = internals::Record {
            data: Vec::new(),
            deleted: false,
            timestamp,
        };
        message.encode(&mut record.data)?;
        self.write_record(row, column, record.as_view())
    }

    pub fn read<T: DeserializeOwned>(
        &self,
        row: &str,
        column: &str,
        timestamp: u64,
    ) -> Option<std::io::Result<T>> {
        let mut record = None;
        let mut latest_ts = 0;
        for table in &self.mtables {
            let _locked = table.read().expect("failed to readlock mtable");
            if let Some(r) = _locked.read(row, column, timestamp) {
                println!("read value from mtable");
                if r.timestamp > latest_ts {
                    latest_ts = r.timestamp;
                    if r.deleted {
                        record = None
                    } else {
                        record = Some(T::decode_owned(&r.data));
                    }
                }
            } else {
                println!("mtable came back empty");
            }
        }

        for table in &self.dtables {
            let _locked = table.read().expect("failed to readlock dtable");
            if let Some(r) = _locked.read(row, column, timestamp) {
                if r.get_timestamp() > latest_ts {
                    latest_ts = r.get_timestamp();
                    if r.get_deleted() {
                        record = None
                    } else {
                        record = Some(T::decode_owned(r.get_data()));
                    }
                }
            }
        }

        record
    }
}

pub fn serialize_key(row: &str, column: &str) -> String {
    format!("{}\x00{}", row, column)
}

pub fn get_record<'a>(
    cell_data: internals::CellDataView<'a>,
    timestamp: u64,
) -> Option<internals::RecordView<'a>> {
    let records = cell_data.get_records();

    // Records are ordered by most recent first. Skip all elements written after the provided time
    // horizon.
    let record = records.iter().find(|r| r.get_timestamp() <= timestamp);

    // It's safe to do this transmute, because the reference from this get(...) is actually tied to
    // the lifetime of the cell data, not the RepeatedField.
    unsafe { std::mem::transmute(record) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write() {
        let mut t = LargeTable::new();
        t.add_mtable();

        let mut buf = Vec::new();
        t.add_journal(&mut buf);

        t.write(String::from("abc"), String::from("def"), 123, "abc def")
            .unwrap();

        // Read from mtable
        let r: String = t.read("abc", "def", 345).unwrap().unwrap();
        assert_eq!(r, "abc def");

        let mut dbuf = Vec::new();
        t.mtables[0]
            .read()
            .unwrap()
            .write_to_dtable(&mut dbuf)
            .unwrap();

        t.mtables.clear();
        t.add_mtable();

        // With no mtables available, should get None
        assert!(t.read::<String>("abc", "def", 345).is_none());

        t.add_dtable_from_bytes(&dbuf).unwrap();

        // Read from dtable, should get value back
        let r: String = t.read("abc", "def", 345).unwrap().unwrap();
        assert_eq!(r, "abc def");

        // Write an updated value
        t.write(String::from("abc"), String::from("def"), 234, "updated")
            .unwrap();

        let r: String = t.read("abc", "def", 345).unwrap().unwrap();
        assert_eq!(r, "updated");
    }
}

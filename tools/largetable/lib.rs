mod dtable;
mod mtable;

use bus::Serialize;

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

    pub fn load_from_journal<R: std::io::Read>(&mut self, reader: R) -> std::io::Result<()> {
        let mut journal = recordio::RecordIOReader::<internals::JournalEntry, _>::new(reader);
        while let Some(entry) = journal.next() {
            let entry = entry?;
            self.write(
                entry.row,
                entry.column,
                entry.record.as_view(),
                entry.timestamp,
            )?;
        }
        Ok(())
    }

    pub fn write(
        &self,
        row: String,
        column: String,
        record: internals::RecordView,
        timestamp: u64,
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
            timestamp,
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
            .write(entry.row, entry.column, entry.record, timestamp);
        Ok(())
    }

    pub fn write_data<T: Serialize>(
        &self,
        row: String,
        column: String,
        timestamp: u64,
        message: T,
    ) -> std::io::Result<()> {
        let mut record = internals::Record {
            data: Vec::new(),
            deleted: false,
        };
        message.encode(&mut record.data)?;
        self.write(row, column, record.as_view(), timestamp)
    }
}

pub fn serialize_key(row: &str, column: &str) -> String {
    format!("{}\x00{}", row, column)
}

pub fn get_record<'a>(
    cell_data: internals::CellDataView<'a>,
    timestamp: u64,
) -> Option<internals::RecordView<'a>> {
    let idx = cell_data
        .get_timestamps()
        .iter()
        .take_while(|t| *t > timestamp)
        .count();

    // It's safe to do this transmute, because the reference from this get(...) is actually tied to
    // the lifetime of the cell data, not the RepeatedField.
    unsafe { std::mem::transmute(cell_data.get_records().get(idx)) }
}

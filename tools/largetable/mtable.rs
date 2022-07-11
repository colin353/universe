use std::collections::BTreeMap;

pub struct MTable {
    tree: BTreeMap<MTableKey, internals::Record>,

    pub memory_usage: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MTableKey {
    row: String,
    column: String,
    timestamp: u64,
}

impl Ord for MTableKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let row_ord = self.row.cmp(&other.row);
        if row_ord != std::cmp::Ordering::Equal {
            return row_ord;
        }

        let col_ord = self.column.cmp(&other.column);
        if col_ord != std::cmp::Ordering::Equal {
            return col_ord;
        }

        // NOTE: timestamp ordering is reversed
        other.timestamp.cmp(&self.timestamp)
    }
}

impl PartialOrd for MTableKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl MTable {
    pub fn new() -> Self {
        Self {
            tree: BTreeMap::new(),
            memory_usage: 0,
        }
    }

    pub fn write(
        &mut self,
        row: String,
        column: String,
        record: internals::Record,
        timestamp: u64,
    ) {
        self.memory_usage += record.data.len();
        self.tree.insert(
            MTableKey {
                row,
                column,
                timestamp,
            },
            record,
        );
    }

    pub fn write_to_dtable<W: std::io::Write>(&self, writer: W) -> std::io::Result<()> {
        let mut dtable = sstable::SSTableBuilder::<internals::CellData, W>::new(writer);
        let mut cell_data = internals::CellData::new();
        let mut working_key = None;
        for (key, value) in self.tree.iter() {
            if let Some((r, c)) = working_key {
                if r != key.row || c != key.column {
                    dtable.write_ordered(&crate::serialize_key(r, c), cell_data)?;
                    cell_data = internals::CellData::new();
                    working_key = Some((&key.row, &key.column));
                }
            } else {
                working_key = Some((&key.row, &key.column));
            }

            cell_data.timestamps.push(key.timestamp);
            cell_data.records.push(value.clone());
        }

        if let Some((r, c)) = working_key {
            dtable.write_ordered(&crate::serialize_key(r, c), cell_data)?;
        }
        dtable.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_to_disk() {
        let mut m = MTable::new();
        m.write(
            String::from("aaa"),
            String::from("bbb"),
            internals::Record {
                data: vec![0x1, 0x2, 0x3],
                deleted: false,
            },
            1234,
        );
        m.write(
            String::from("aaa"),
            String::from("bbb"),
            internals::Record {
                data: vec![],
                deleted: true,
            },
            2345,
        );
        m.write(
            String::from("bbb"),
            String::from("ccc"),
            internals::Record {
                data: vec![0x1, 0x2, 0x3],
                deleted: false,
            },
            12345,
        );

        let mut buf = Vec::new();
        m.write_to_dtable(&mut buf).unwrap();

        let reader = sstable::SSTableReader::from_bytes(&buf).unwrap();
        let cell: internals::CellData = reader.get(&crate::serialize_key("aaa", "bbb")).unwrap();

        assert_eq!(&cell.timestamps, &[2345, 1234]);
        assert_eq!(cell.records[0].deleted, true);

        let cell = reader.get(&crate::serialize_key("bbb", "ccc")).unwrap();

        assert_eq!(&cell.timestamps, &[12345]);
        assert_eq!(cell.records[0].data, &[0x1, 0x2, 0x3]);
    }
}

use std::borrow::Cow;
use std::collections::BTreeMap;

pub struct MTable {
    tree: BTreeMap<MTableKey<'static>, internals::Record>,

    pub memory_usage: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MTableKey<'a> {
    row: Cow<'a, str>,
    column: Cow<'a, str>,
    timestamp: u64,
}

impl<'a> Ord for MTableKey<'a> {
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

impl<'a> PartialOrd for MTableKey<'a> {
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

    pub fn write(&mut self, row: String, column: String, record: internals::Record) {
        self.memory_usage += record.data.len();
        self.tree.insert(
            MTableKey {
                row: Cow::Owned(row),
                column: Cow::Owned(column),
                timestamp: record.timestamp,
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
                if r != key.row.as_ref() || c != key.column.as_ref() {
                    dtable.write_ordered(&crate::serialize_key(r, c), cell_data)?;
                    cell_data = internals::CellData::new();
                    working_key = Some((&key.row, &key.column));
                }
            } else {
                working_key = Some((&key.row, &key.column));
            }
            cell_data.records.push(value.clone());
        }

        if let Some((r, c)) = working_key {
            dtable.write_ordered(&crate::serialize_key(r, c), cell_data)?;
        }
        dtable.finish()
    }

    pub fn read<'a>(
        &'a self,
        row: &'a str,
        column: &'a str,
        timestamp: u64,
    ) -> Option<&'a internals::Record> {
        let mut iter = self.tree.range((
            std::collections::Bound::Included(&MTableKey {
                row: Cow::Borrowed(row),
                column: Cow::Borrowed(column),
                timestamp,
            }),
            std::collections::Bound::Unbounded,
        ));

        let value = iter.next()?;
        if value.0.column != column || value.0.row != row {
            return None;
        }

        Some(value.1)
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
                timestamp: 1234,
            },
        );
        m.write(
            String::from("aaa"),
            String::from("bbb"),
            internals::Record {
                data: vec![],
                deleted: true,
                timestamp: 2345,
            },
        );
        m.write(
            String::from("bbb"),
            String::from("ccc"),
            internals::Record {
                data: vec![0x1, 0x2, 0x3],
                deleted: false,
                timestamp: 3456,
            },
        );

        let mut buf = Vec::new();
        m.write_to_dtable(&mut buf).unwrap();

        let reader = sstable::SSTableReader::from_bytes(&buf).unwrap();
        let cell: internals::CellData = reader.get(&crate::serialize_key("aaa", "bbb")).unwrap();

        assert_eq!(cell.records[0].timestamp, 2345);
        assert_eq!(cell.records[0].deleted, true);

        let cell = reader.get(&crate::serialize_key("bbb", "ccc")).unwrap();

        assert_eq!(cell.records[0].timestamp, 3456);
        assert_eq!(cell.records[0].data, &[0x1, 0x2, 0x3]);
    }
}

pub struct DTable<'a> {
    table: sstable::SSTableReader<internals::CellDataView<'a>>,
}

impl<'a> DTable<'a> {
    pub fn from_file(f: std::fs::File) -> std::io::Result<Self> {
        Ok(Self {
            table: sstable::SSTableReader::from_file(f)?,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        Ok(Self {
            table: sstable::SSTableReader::from_bytes(bytes)?,
        })
    }

    pub fn read<'b>(
        &'b self,
        row: &str,
        column: &str,
        timestamp: u64,
    ) -> Option<internals::RecordView<'b>> {
        let cell = self.table.get(&crate::serialize_key(row, column))?;
        let record = crate::get_record(cell, timestamp)?;
        Some(record)
    }

    pub fn iter_at(
        &'a self,
        filter: sstable::Filter<'a>,
        timestamp: u64,
    ) -> impl Iterator<Item = (String, internals::RecordView<'a>)> {
        DTableIterator {
            iter: self.table.iter_ek_at(filter),
            timestamp,
        }
    }
}

struct DTableIterator<'a> {
    iter: sstable::SSTableEKIterator<'a, internals::CellDataView<'a>>,
    timestamp: u64,
}

impl<'a> Iterator for DTableIterator<'a> {
    type Item = (String, internals::RecordView<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        let (key, cell_data) = self.iter.next()?;
        let record = match crate::get_record(cell_data, self.timestamp) {
            Some(r) => r,
            None => return self.next(),
        };

        // To avoid an extra allocation, let's try to figure out the column directly
        // from the encoded key.
        let prefix = &self.iter.prefix[0..key.prefix];
        let column = if let Some(idx) = prefix.find("\x00") {
            format!("{}{}", &prefix[idx + 1..], key.suffix)
        } else {
            String::from(
                key.suffix
                    .rsplit("\x00")
                    .next()
                    .expect("split always yields at least one value"),
            )
        };

        Some((column, record))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtable_read() {
        let mut m = crate::mtable::MTable::new();
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
                data: vec![0x2, 0x3, 0x4],
                deleted: false,
                timestamp: 12345,
            },
        );

        let mut buf = Vec::new();
        m.write_to_dtable(&mut buf).unwrap();

        let d = DTable::from_bytes(&buf).unwrap();
        assert!(d.read("aaa", "bbb", 0).is_none());
        assert_eq!(
            d.read("aaa", "bbb", 2222).unwrap().get_data(),
            &[0x1, 0x2, 0x3]
        );
        assert_eq!(d.read("aaa", "bbb", 3333).unwrap().get_deleted(), true);

        let mut iter = d.iter_at(sstable::Filter::all(), 2222).map(|(_, v)| v);
        assert_eq!(iter.next().unwrap().get_data(), &[0x1, 0x2, 0x3]);
        assert!(iter.next().is_none());

        let mut iter = d.iter_at(sstable::Filter::all(), 99999).map(|(_, v)| v);
        assert_eq!(iter.next().unwrap().get_deleted(), true);
        assert_eq!(iter.next().unwrap().get_data(), &[0x2, 0x3, 0x4]);
        assert!(iter.next().is_none());
    }
}

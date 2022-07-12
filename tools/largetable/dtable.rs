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

    pub fn read(
        &self,
        row: &str,
        column: &str,
        timestamp: u64,
    ) -> Option<std::io::Result<internals::Record>> {
        let cell = self.table.get(&crate::serialize_key(row, column))?;
        let record = crate::get_record(cell, timestamp)?;
        Some(record.to_owned())
    }

    pub fn iter_at(
        &'a self,
        filter: sstable::Filter<'a>,
        timestamp: u64,
    ) -> impl Iterator<Item = (sstable::EncodedKey<'a>, internals::RecordView<'a>)> {
        self.table
            .iter_at(filter)
            .filter_map(move |(key, cell_data)| {
                Some((key, crate::get_record(cell_data, timestamp)?))
            })
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
                data: vec![0x2, 0x3, 0x4],
                deleted: false,
            },
            12345,
        );

        let mut buf = Vec::new();
        m.write_to_dtable(&mut buf).unwrap();

        let d = DTable::from_bytes(&buf).unwrap();
        assert!(d.read("aaa", "bbb", 0).is_none());
        assert_eq!(
            d.read("aaa", "bbb", 2222).unwrap().unwrap().data,
            &[0x1, 0x2, 0x3]
        );
        assert_eq!(d.read("aaa", "bbb", 3333).unwrap().unwrap().deleted, true);

        let mut iter = d.iter_at(sstable::Filter::all(), 99999).map(|(k, v)| v);
        assert_eq!(iter.next().unwrap().get_data(), &[0x1, 0x2, 0x3]);
        assert_eq!(iter.next().unwrap().get_data(), &[0x2, 0x3, 0x4]);
        assert!(iter.next().is_none());
    }
}

use byteorder::{ByteOrder, LittleEndian};
use primitive::{Primitive, Serializable};

use crate::index;

pub struct MMappedSSTableReader<T> {
    index: sstable_proto_rust::Index,
    dtable: mmap::Mmap,
    data_type: std::marker::PhantomData<T>,
    index_offset: usize,
}

impl<T: Serializable + Default> MMappedSSTableReader<T> {
    pub fn new(file: std::fs::File) -> Result<Self, std::io::Error> {
        let dtable = unsafe { mmap::MmapOptions::new().map(&file)? };

        let index_offset =
            LittleEndian::read_u64(&dtable[dtable.len() - 16..dtable.len() - 8]) as usize;
        let index_size = LittleEndian::read_u64(&dtable[dtable.len() - 8..]) as usize;

        let index =
            match protobuf::parse_from_bytes(&dtable[index_offset..index_offset + index_size]) {
                Ok(i) => i,
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "unable to parse sstable index",
                    ))
                }
            };

        Ok(Self {
            dtable,
            index,
            data_type: std::marker::PhantomData,
            index_offset,
        })
    }

    pub fn get(&self, key: &str) -> std::io::Result<Option<T>> {
        let mut offset = match index::get_block(&self.index, key) {
            Some(block) => block.get_offset(),
            None => return Ok(None),
        } as usize;

        loop {
            let (key_entry, new_offset) = match self.read_at(offset) {
                Ok(Some((de, idx))) => (de, idx),
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            };

            offset = new_offset;

            if key_entry.get_key() > key {
                return Ok(None);
            } else if key == key_entry.get_key() {
                let mut value = T::default();
                value.read_from_bytes(&key_entry.get_value())?;
                return Ok(Some(value));
            }
        }
    }

    pub fn read_at(
        &self,
        offset: usize,
    ) -> std::io::Result<Option<(sstable_proto_rust::DataEntry, usize)>> {
        if offset >= self.index_offset {
            return Ok(None);
        }

        let size = LittleEndian::read_u32(&self.dtable[offset..offset + 4]) as usize;
        // Increment the dtable offset. We've read 4 bytes to read the size, plus we're about
        // to read size bytes, so we want to advance it by size + 4.
        let new_offset = offset + size + 4;
        Ok(Some((
            protobuf::parse_from_bytes(&self.dtable[offset + 4..new_offset]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unable to parse KeyEntry protobuf in sstable: {}", e),
                )
            })?,
            new_offset,
        )))
    }
}

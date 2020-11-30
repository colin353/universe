const BLOCK_SIZE: u64 = 65536;
const VERSION: u16 = 0;

mod index;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use primitive::Serializable;
use protobuf::Message;
use std::io::{Error, Result};

pub struct SSTableBuilder<T, W> {
    index: sstable_proto_rust::Index,
    writer: W,
    last_key: String,
    bytes_written: u64,
    _marker: std::marker::PhantomData<T>,
}

impl<W: std::io::Write, T: Serializable> SSTableBuilder<T, W> {
    pub fn new(writer: W) -> Self {
        Self {
            index: sstable_proto_rust::Index::new(),
            writer,
            last_key: String::new(),
            bytes_written: 0,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn write_ordered(&mut self, key: &str, value: T) -> Result<()> {
        let mut buffer = Vec::new();
        value.write(&mut buffer)?;
        self.write_raw(key, &buffer)
    }

    pub fn write_raw(&mut self, key: &str, value: &[u8]) -> Result<()> {
        assert!(
            self.bytes_written == 0 || key >= self.last_key.as_str(),
            format!(
                "Keys must be written in order to the SSTable!\n `{}` (written) < `{}` (previous)",
                key, self.last_key
            )
        );
        let key_bytes = key.as_bytes();
        self.writer
            .write_u16::<LittleEndian>(key_bytes.len() as u16);
        self.writer.write_all(key_bytes);
        self.writer.write_u32::<LittleEndian>(value.len() as u32);
        self.writer.write_all(value);

        // If we've written the first entry, or we've crossed a block boundary while writing, write
        // an index entry
        let length = (2 + 4 + key_bytes.len() + value.len()) as u64;
        if self.bytes_written == 0
            || length >= BLOCK_SIZE
            || (self.bytes_written + length) % BLOCK_SIZE < (self.bytes_written % BLOCK_SIZE)
        {
            let mut key_entry = sstable_proto_rust::KeyEntry::new();
            key_entry.set_key(key.to_string());
            key_entry.set_offset(self.bytes_written);
            self.index.mut_pointers().push(key_entry);
        }

        self.bytes_written += length;

        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.index.write_to_writer(&mut self.writer).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unexpectedly unable to write index during finish(): {}", e),
            )
        })?;

        self.writer.write_u16::<LittleEndian>(VERSION)?;
        self.writer.write_u64::<LittleEndian>(self.bytes_written)?;
        self.writer
            .write_u32::<LittleEndian>(self.index.compute_size())?;

        Ok(())
    }
}

pub struct SSTableReader<T> {
    index: sstable_proto_rust::Index,
    dtable: mmap::Mmap,
    _marker: std::marker::PhantomData<T>,
    index_offset: usize,
    version: u16,
    offset: usize,
}

impl<T: Serializable + Default> Iterator for SSTableReader<T> {
    type Item = (String, T);
    fn next(&mut self) -> Option<(String, T)> {
        let (k, v, idx) = match self.read_at(self.offset).unwrap() {
            Some((k, v, idx)) => {
                let mut value = T::default();
                value.read_from_bytes(v).unwrap();
                (k.to_string(), value, idx)
            }
            None => return None,
        };

        self.offset = idx;
        Some((k, v))
    }
}

impl<T: Serializable + Default> SSTableReader<T> {
    pub fn new(file: std::fs::File) -> Result<Self> {
        let dtable = unsafe { mmap::MmapOptions::new().map(&file)? };

        let version = LittleEndian::read_u16(&dtable[dtable.len() - 14..dtable.len() - 12]);
        let index_offset =
            LittleEndian::read_u64(&dtable[dtable.len() - 12..dtable.len() - 4]) as usize;
        let index_size = LittleEndian::read_u32(&dtable[dtable.len() - 4..]) as usize;

        let index =
            match protobuf::parse_from_bytes(&dtable[index_offset..index_offset + index_size]) {
                Ok(i) => i,
                Err(_) => {
                    return Err(Error::new(
                        std::io::ErrorKind::InvalidData,
                        "unable to parse sstable index",
                    ))
                }
            };

        Ok(Self {
            dtable,
            index,
            _marker: std::marker::PhantomData,
            index_offset,
            version,
            offset: 0,
        })
    }

    pub fn get(&self, key: &str) -> Result<Option<T>> {
        let mut offset = match index::get_block(&self.index, key) {
            Some(block) => block.get_offset(),
            None => return Ok(None),
        } as usize;

        loop {
            let (found_key, value, new_offset) = match self.read_at(offset) {
                Ok(Some((found_key, value, idx))) => (found_key, value, idx),
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            };

            offset = new_offset;

            if found_key > key {
                return Ok(None);
            } else if key == found_key {
                let mut parsed_value = T::default();
                parsed_value.read_from_bytes(value)?;
                return Ok(Some(parsed_value));
            }
        }
    }

    pub fn read_at(&self, mut offset: usize) -> Result<Option<(&str, &[u8], usize)>> {
        if offset >= self.index_offset {
            return Ok(None);
        }

        let key_length = LittleEndian::read_u16(&self.dtable[offset..offset + 2]) as usize;
        let key = unsafe {
            std::str::from_utf8_unchecked(&self.dtable[offset + 2..offset + 2 + key_length])
        };

        offset += key_length + 2;

        let value_length = LittleEndian::read_u32(&self.dtable[offset..offset + 4]) as usize;
        let value = &self.dtable[offset + 4..offset + 4 + value_length];

        offset += value_length + 4;

        Ok(Some((key, value, offset)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitive::Primitive;
    use std::io::Seek;

    #[test]
    fn write_a_very_long_sstable() {
        let mut f = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut f);
            for x in 0..100 {
                t.write_ordered(format!("{:9}", x).as_str(), Primitive(x))
                    .unwrap();
            }
            t.finish().unwrap();
        }
    }

    #[test]
    fn read_and_write_sstable() {
        let mut f = std::fs::File::create("/tmp/test.sstable").unwrap();
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut f);
            for x in 0..100_000 {
                t.write_ordered(format!("{:9}", x).as_str(), Primitive(x))
                    .unwrap();
            }
            t.finish().unwrap();
        }

        let f = std::fs::File::open("/tmp/test.sstable").unwrap();
        let mut r = SSTableReader::<Primitive<i64>>::new(f).unwrap();

        // 100k entries, approx 19 bytes per entry (9 bytes of string, 6 bytes of size/alignment, 3
        // bytes of integers) / 65536 = 29
        assert_eq!(r.index.get_pointers().len(), 29);

        for x in 0..100_000 {
            println!("x = {}", x);
            assert_eq!(r.next().unwrap(), (format!("{:9}", x), Primitive(x)));
        }
        assert_eq!(r.next(), None);
    }
}

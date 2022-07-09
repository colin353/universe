use bus::{Deserialize, Serialize};

use std::convert::TryInto;

mod bloom_filter;

const BLOCK_SIZE: u64 = 65536;

pub struct SSTableBuilder<T, W> {
    index: sstable_bus::Index,
    writer: W,
    last_key: String,
    bytes_written: u64,
    record_count: u64,
    bloom_filter: bloom_filter::BloomFilterBuilder,
    _marker: std::marker::PhantomData<T>,
}

pub struct SSTableReader<T> {
    index: sstable_bus::Index,
    mmap: mmap::Mmap,
    footer_offset: usize,

    // NOTE: static due to unsafe alias of the mmap
    bloom_filter: bloom_filter::BloomFilter<'static>,
    record_count: usize,
    version: sstable_bus::Version,

    _marker: std::marker::PhantomData<T>,
}

impl<W: std::io::Write, T: Serialize> SSTableBuilder<T, W> {
    pub fn new(writer: W) -> Self {
        Self::with_bloom_filter(writer, bloom_filter::BloomFilterBuilder::small())
    }

    pub fn with_bloom_filter(writer: W, bloom_filter: bloom_filter::BloomFilterBuilder) -> Self {
        Self {
            index: sstable_bus::Index::new(),
            writer,
            last_key: String::new(),
            bytes_written: 0,
            record_count: 0,
            _marker: std::marker::PhantomData,
            bloom_filter,
        }
    }

    pub fn write_ordered(&mut self, key: &str, value: T) -> std::io::Result<()> {
        let shared_prefix = key
            .chars()
            .zip(self.last_key.chars())
            .take_while(|(x, y)| x == y)
            .count();

        let mut record = sstable_bus::Record {
            shared_prefix: shared_prefix as u16,
            key_suffix: key[shared_prefix..].to_owned(),
            data_length: 0,
        };

        let mut buf = Vec::new();
        record.data_length = value.encode(&mut buf)? as u32;
        let record_length = record.encode(&mut buf)? as u32;

        self.writer.write_all(&record_length.to_le_bytes())?;
        self.writer.write_all(&buf[record.data_length as usize..])?;
        self.writer
            .write_all(&buf[0..record.data_length as usize])?;

        let length = buf.len() as u64 + 4;

        if self.bytes_written == 0 || (self.bytes_written % BLOCK_SIZE) + length > BLOCK_SIZE {
            self.index.keys.push(sstable_bus::BlockKey {
                key: key.to_owned(),
                offset: self.bytes_written,
            })
        }

        self.bytes_written += length;
        self.record_count += 1;
        self.bloom_filter.insert(key);

        Ok(())
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        let footer = sstable_bus::Footer {
            bloom_filter: self.bloom_filter.optimize(),
            index: self.index,
            record_count: self.record_count,
            version: sstable_bus::Version::V0,
        };

        let footer_length = footer.encode(&mut self.writer)? as u32;
        self.writer.write_all(&footer_length.to_le_bytes())?;

        Ok(())
    }
}

impl<'a, T: 'a + Deserialize<'a>> SSTableReader<T> {
    pub fn from_file(file: std::fs::File) -> std::io::Result<Self> {
        Self::from_mmap(unsafe { mmap::MmapOptions::new().map(&file)? })
    }

    pub fn from_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        let mut map = mmap::MmapMut::map_anon(bytes.len())?;
        map.copy_from_slice(bytes);
        Self::from_mmap(map.make_read_only()?)
    }

    pub fn from_filename(filename: &str) -> std::io::Result<Self> {
        let f = std::fs::File::open(filename)?;
        Self::from_file(f)
    }

    pub fn from_mmap(m: mmap::Mmap) -> std::io::Result<Self> {
        let footer_length = u32::from_le_bytes(match m[m.len() - 4..].try_into() {
            Ok(d) => d,
            Err(_) => return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)),
        });

        let footer_offset = m.len() - 4 - footer_length as usize;
        let footer = sstable_bus::FooterView::from_bytes(&m[footer_offset..m.len() - 4])?;

        let version = footer.get_version();
        let record_count = footer.get_record_count() as usize;
        let index = footer.get_index().to_owned()?;

        let bf = footer.get_bloom_filter();
        let bf_len = bf.len();
        let slice: &'static [u8] = unsafe { std::slice::from_raw_parts(bf.as_ptr(), bf_len) };

        let bloom_filter = bloom_filter::BloomFilter::from_bytes(slice);

        Ok(Self {
            index,
            mmap: m,
            bloom_filter,
            footer_offset,
            record_count,
            version,

            _marker: std::marker::PhantomData,
        })
    }

    fn get_block(&self, key: &str) -> Option<&sstable_bus::BlockKey> {
        if self.index.keys.is_empty() {
            return None;
        }

        let mut idx = match self
            .index
            .keys
            .binary_search_by_key(&key, |b| b.key.as_str())
        {
            // If the index is zero, that means our key is less than any block key, which means
            // it doesn't exist.
            Ok(0) | Err(0) => return None,
            Ok(x) | Err(x) => x - 1,
        };

        // If our target key is exactly equal to a block key, seek backward until we find a block
        // key less than the target key. This can happen because `binary_search_by_key` can return
        // any equal value, but we want the first one.
        while idx > 0 && self.index.keys[idx].key == key {
            idx -= 1;
        }

        Some(&self.index.keys[idx])
    }

    pub fn get(&self, key: &str) -> Option<T> {
        // Check the bloom filter to see if the key exists
        if !self.bloom_filter.contains(key) {
            return None;
        }

        let block = match self.get_block(key) {
            Some(b) => b,
            None => return None,
        };

        for item in self.iter_at_offset(block.offset as usize, block.key.clone()) {
            return Some(item);
        }

        None
    }

    pub fn iter_at_offset(&self, offset: usize, prefix: String) -> SSTableIterator<'a, T> {
        SSTableIterator {
            reader: self,
            offset,
            prefix,
        }
    }

    pub fn iter(&'a self) -> SSTableIterator<'a, T> {
        let prefix = match self.index.keys.get(0) {
            Some(k) => k.key.to_owned(),
            None => String::new(),
        };

        SSTableIterator {
            reader: self,
            offset: 0,
            prefix,
        }
    }
}

pub struct SSTableIterator<'a, T> {
    reader: &'a SSTableReader<T>,
    offset: usize,
    prefix: String,
}

impl<'a, T: Deserialize<'a>> Iterator for SSTableIterator<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let data = &self.reader.mmap[self.offset as usize..self.reader.footer_offset];
        if self.offset >= data.len() {
            return None;
        }

        let record_length = u32::from_le_bytes(match data[self.offset..4].try_into() {
            Ok(d) => d,
            Err(_) => return None,
        });
        let record_end = record_length as usize + 4;
        let record = sstable_bus::RecordView::from_bytes(&data[4..record_end]).ok()?;
        let data_length = record.get_data_length() as usize;

        self.offset += record_end + data_length;
        if self.offset >= data.len() {
            return None;
        }
        let payload = T::decode(&data[record_end..self.offset]).ok()?;

        Some(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_sstable() {
        let mut buf = Vec::new();
        let mut builder = SSTableBuilder::new(&mut buf);
        builder.write_ordered("abc", "apple").unwrap();
        builder.write_ordered("bcd", "strawberry").unwrap();
        builder.write_ordered("cde", "pineapple").unwrap();
        builder.finish().unwrap();

        let reader = SSTableReader::<&str>::from_bytes(&buf).unwrap();
        assert_eq!(
            reader.get_block("strawberry").unwrap(),
            &sstable_bus::BlockKey {
                key: String::from("abc"),
                offset: 0,
            }
        );

        assert_eq!(reader.get("bcd").unwrap(), "a");
    }
}

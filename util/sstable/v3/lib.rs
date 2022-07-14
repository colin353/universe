use bus::{Deserialize, Serialize};

use std::convert::TryInto;

mod bloom_filter;

const BLOCK_SIZE: u64 = 65536;
const PREFIX_RESET: u64 = 64;

pub struct SSTableBuilder<T, W> {
    index: sstable_bus::Index,
    writer: W,
    prev_key: String,
    shared_prefix: String,
    bytes_written: u64,
    record_count: u64,
    bloom_filter: bloom_filter::BloomFilterBuilder,
    contains_duplicate_keys: bool,
    _marker: std::marker::PhantomData<T>,
}

pub struct SSTableReader<T> {
    index: sstable_bus::Index,
    mmap: mmap::Mmap,
    footer_offset: usize,

    // NOTE: static due to unsafe alias of the mmap
    bloom_filter: bloom_filter::BloomFilter<'static>,
    pub record_count: usize,
    pub version: sstable_bus::Version,
    contains_duplicate_keys: bool,

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
            shared_prefix: String::new(),
            bytes_written: 0,
            prev_key: String::new(),
            record_count: 0,
            _marker: std::marker::PhantomData,
            bloom_filter,
            contains_duplicate_keys: false,
        }
    }

    pub fn write_ordered(&mut self, key: &str, value: T) -> std::io::Result<()> {
        if key < &self.prev_key {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "tried to write keys out of order, {:?} > {:?}",
                    key, self.prev_key
                ),
            ));
        }

        if key == self.prev_key {
            self.contains_duplicate_keys = true
        }

        let used_prefix = if self.record_count % PREFIX_RESET == 0 {
            0
        } else {
            key.chars()
                .zip(self.shared_prefix.chars())
                .take_while(|(x, y)| x == y)
                .count()
        };

        if used_prefix == 0 {
            self.shared_prefix = key.to_owned();
        }

        let mut record = sstable_bus::Record {
            prefix: used_prefix as u16,
            suffix: key[used_prefix..].to_owned(),
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
        self.prev_key = key.to_owned();

        Ok(())
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        let footer = sstable_bus::Footer {
            bloom_filter: self.bloom_filter.optimize(),
            index: self.index,
            record_count: self.record_count,
            version: sstable_bus::Version::V0,
            contains_duplicate_keys: self.contains_duplicate_keys,
        };

        let footer_length = footer.encode(&mut self.writer)? as u32;
        self.writer.write_all(&footer_length.to_le_bytes())?;

        Ok(())
    }
}

impl<'a, T: Deserialize<'a>> SSTableReader<T> {
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

        if (footer_length + 4) as usize > m.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }

        let footer_offset = m.len() - 4 - footer_length as usize;
        let footer = sstable_bus::FooterView::from_bytes(&m[footer_offset..m.len() - 4])?;

        let version = footer.get_version();
        let record_count = footer.get_record_count() as usize;
        let index = footer.get_index().to_owned()?;
        let contains_duplicate_keys = footer.get_contains_duplicate_keys();

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
            contains_duplicate_keys,

            _marker: std::marker::PhantomData,
        })
    }

    fn get_block(&self, key: &str) -> Option<&sstable_bus::BlockKey> {
        if self.index.keys.is_empty() {
            println!("there are no keys");
            return None;
        }

        let mut idx = match self
            .index
            .keys
            .binary_search_by_key(&key, |b| b.key.as_str())
        {
            // If the index is zero, that means our key is less than any block key, which means
            // it doesn't exist.
            //
            // NOTE to self: this is wrong for range-based seeking, e.g. when you want to seek to
            // the "" key to iterate everything!
            Ok(0) | Err(0) => {
                return Some(&self.index.keys[0]);
            }
            Ok(x) | Err(x) => x - 1,
        };

        // If our target key is exactly equal to a block key, seek backward until we find a block
        // key less than the target key. This can happen because `binary_search_by_key` can return
        // any equal value, but we want the first one.
        if self.contains_duplicate_keys {
            while idx > 0 && self.index.keys[idx].key == key {
                idx -= 1;
            }
        }

        Some(&self.index.keys[idx])
    }

    pub fn get(&'a self, key: &str) -> Option<T> {
        // Check the bloom filter to see if the key exists
        if !self.bloom_filter.contains(key) {
            return None;
        }

        let block = match self.get_block(key) {
            Some(b) => b,
            None => {
                return None;
            }
        };

        let mut iter = self.iter_ek_at_offset(block.offset as usize, &block.key);

        let mut first = true;
        while let Some((encoded_key, item)) = iter.next() {
            if first {
                first = false;
            }

            match encoded_key.cmp(&iter.prefix, key) {
                std::cmp::Ordering::Equal => return Some(item),
                std::cmp::Ordering::Greater => return None,
                _ => (),
            }
        }

        None
    }

    pub fn iter<'b>(&'b self) -> SSTableIterator<'b, T> {
        SSTableIterator {
            inner: self.iter_ek(),
        }
    }

    pub fn iter_at<'b>(&'b self, filter: Filter<'b>) -> SSTableIterator<'b, T> {
        SSTableIterator {
            inner: self.iter_ek_at(filter),
        }
    }

    pub fn iter_at_offset<'b>(&'b self, offset: usize, prefix: &'b str) -> SSTableIterator<'b, T> {
        SSTableIterator {
            inner: self.iter_ek_at_offset(offset, prefix),
        }
    }

    pub fn iter_ek_at<'b>(&'b self, filter: Filter<'b>) -> SSTableEKIterator<'b, T> {
        let block = match self.get_block(filter.start()) {
            Some(b) => b,
            None => {
                return SSTableEKIterator {
                    reader: self,
                    offset: usize::MAX,
                    prefix: "",
                    filter: Filter::all(),
                };
            }
        };

        SSTableEKIterator {
            reader: self,
            offset: block.offset as usize,
            prefix: &block.key,
            filter,
        }
    }

    pub fn iter_ek_at_offset<'b>(
        &'b self,
        offset: usize,
        prefix: &'b str,
    ) -> SSTableEKIterator<'b, T> {
        SSTableEKIterator {
            reader: self,
            offset,
            prefix,
            filter: Filter::all(),
        }
    }

    pub fn iter_ek<'b>(&'b self) -> SSTableEKIterator<'b, T> {
        let prefix = match self.index.keys.get(0) {
            Some(k) => &k.key,
            None => "",
        };

        SSTableEKIterator {
            reader: self,
            offset: 0,
            prefix,
            filter: Filter::all(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Filter<'a> {
    pub spec: &'a str,
    pub min: &'a str,
    pub max: &'a str,
}

impl<'a> Filter<'a> {
    pub fn all() -> Self {
        Self {
            spec: "",
            min: "",
            max: "",
        }
    }

    pub fn from_spec(spec: &'a str) -> Self {
        Self {
            spec,
            min: "",
            max: "",
        }
    }

    pub fn starting_at(min: &'a str) -> Self {
        Self {
            spec: "",
            min,
            max: "",
        }
    }

    pub fn until(max: &'a str) -> Self {
        Self {
            spec: "",
            min: "",
            max,
        }
    }

    pub fn from_range(min: &'a str, max: &'a str) -> Self {
        Self { spec: "", min, max }
    }

    pub fn start(&self) -> &str {
        std::cmp::max(self.spec, self.min)
    }

    pub fn matches(&self, key: &str) -> std::cmp::Ordering {
        if key < self.spec || key < self.min {
            return std::cmp::Ordering::Less;
        }

        if !key.starts_with(self.spec) {
            return std::cmp::Ordering::Greater;
        }

        if !self.max.is_empty() && key >= self.max {
            return std::cmp::Ordering::Greater;
        }

        std::cmp::Ordering::Equal
    }

    pub fn matches_encoded(&self, key: &EncodedKey, prefix: &str) -> std::cmp::Ordering {
        if key.cmp(prefix, self.spec) == std::cmp::Ordering::Less
            || key.cmp(prefix, self.min) == std::cmp::Ordering::Less
        {
            return std::cmp::Ordering::Less;
        }

        if !key.starts_with(prefix, self.spec) {
            return std::cmp::Ordering::Greater;
        }

        if !self.max.is_empty() && key.cmp(prefix, self.max) != std::cmp::Ordering::Less {
            return std::cmp::Ordering::Greater;
        }

        std::cmp::Ordering::Equal
    }
}

pub struct SSTableIterator<'a, T> {
    inner: SSTableEKIterator<'a, T>,
}

pub struct SSTableEKIterator<'a, T> {
    reader: &'a SSTableReader<T>,
    offset: usize,
    prefix: &'a str,
    filter: Filter<'a>,
}

#[derive(Debug)]
pub struct EncodedKey<'a> {
    prefix: usize,
    suffix: &'a str,
}

impl<'a> EncodedKey<'a> {
    fn new(prefix: usize, suffix: &'a str) -> Self {
        EncodedKey { prefix, suffix }
    }

    pub fn as_string(&self, prefix: &str) -> String {
        format!("{}{}", &prefix[0..self.prefix], self.suffix)
    }

    pub fn starts_with(&self, prefix: &str, other: &str) -> bool {
        if other.len() > self.prefix + self.suffix.len() {
            return false;
        }

        let (left, right) = other.split_at(std::cmp::min(self.prefix, other.len()));

        if !prefix[0..self.prefix].starts_with(left) {
            return false;
        }

        self.suffix.starts_with(right)
    }

    pub fn equals(&self, prefix: &str, other: &str) -> bool {
        if self.prefix + self.suffix.len() != other.len() {
            return false;
        }

        if !other.starts_with(&prefix[0..self.prefix]) {
            return false;
        }

        &other[self.prefix..] == self.suffix
    }

    pub fn cmp(&self, prefix: &str, other: &str) -> std::cmp::Ordering {
        if !other.starts_with(&prefix[0..self.prefix]) {
            return match &prefix[0..self.prefix] > other {
                true => std::cmp::Ordering::Greater,
                false => std::cmp::Ordering::Less,
            };
        }

        self.suffix.cmp(&other[self.prefix..])
    }
}

impl<'a, T: Deserialize<'a>> Iterator for SSTableIterator<'a, T> {
    type Item = (String, T);
    fn next(&mut self) -> Option<Self::Item> {
        let (ek, value) = self.inner.next()?;
        Some((ek.as_string(self.inner.prefix), value))
    }
}

impl<'a, T: Deserialize<'a>> Iterator for SSTableEKIterator<'a, T> {
    type Item = (EncodedKey<'a>, T);
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.reader.footer_offset {
            return None;
        }

        let data = &self.reader.mmap[self.offset as usize..self.reader.footer_offset];

        let record_length = u32::from_le_bytes(match data[0..4].try_into() {
            Ok(d) => d,
            Err(_) => return None,
        });
        let record_end = record_length as usize + 4;
        let record = sstable_bus::RecordView::from_bytes(&data[4..record_end]).ok()?;
        let data_length = record.get_data_length() as usize;

        let suffix = record.get_suffix();

        // NOTE: this is safe, because the suffix string is actually a pointer into the
        // mmap, which will live for the lifetime of the reader, 'a. But the compiler
        // doesn't understand that.
        let suffix_underlying: &'a str = unsafe {
            let slice = std::slice::from_raw_parts(suffix.as_ptr(), suffix.len());
            std::str::from_utf8_unchecked(slice)
        };

        let encoded_key = if record.get_prefix() == 0 {
            self.prefix = suffix_underlying;
            EncodedKey::new(self.prefix.len(), "")
        } else {
            EncodedKey::new(record.get_prefix() as usize, suffix_underlying)
        };

        self.offset += record_end + data_length;

        if record_end + data_length > data.len() {
            return None;
        }

        return match self.filter.matches_encoded(&encoded_key, self.prefix) {
            std::cmp::Ordering::Less => self.next(),
            std::cmp::Ordering::Greater => None,
            std::cmp::Ordering::Equal => {
                let payload = T::decode(&data[record_end..record_end + data_length]).ok()?;
                Some((encoded_key, payload))
            }
        };
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

        assert_eq!(reader.get("abc").unwrap(), "apple");
        assert_eq!(reader.get("bcd").unwrap(), "strawberry");
        assert_eq!(reader.get("cde").unwrap(), "pineapple");
        assert_eq!(reader.get("qqq"), None);
        assert_eq!(reader.get(""), None);
    }

    #[test]
    fn test_key_compression() {
        let mut buf = Vec::new();
        let mut builder = SSTableBuilder::new(&mut buf);
        builder.write_ordered("abc_1", "apple").unwrap();
        builder.write_ordered("abc_2", "strawberry").unwrap();
        builder.write_ordered("abc_3", "pineapple").unwrap();
        builder.finish().unwrap();

        let reader = SSTableReader::<&str>::from_bytes(&buf).unwrap();
        assert_eq!(reader.get("abc_1").unwrap(), "apple");
        assert_eq!(reader.get("abc_2").unwrap(), "strawberry");
        assert_eq!(reader.get("abc_3").unwrap(), "pineapple");
    }

    #[test]
    fn test_duplicate_keys() {
        let mut buf = Vec::new();
        let mut builder =
            SSTableBuilder::with_bloom_filter(&mut buf, bloom_filter::BloomFilterBuilder::empty());
        for i in 0..10000 {
            builder
                .write_ordered("my-heavily-duplicated-key", format!("{}{}{}", i, i, i))
                .unwrap();
        }
        builder.finish().unwrap();

        let reader = SSTableReader::<&str>::from_bytes(&buf).unwrap();

        assert_eq!(reader.contains_duplicate_keys, true);
        assert_eq!(reader.index.keys.len(), 3);

        let mut iter = reader.iter_ek().map(|(_, v)| v);
        assert_eq!(iter.next().unwrap(), "000");
        assert_eq!(iter.next().unwrap(), "111");
        assert_eq!(iter.next().unwrap(), "222");
        assert_eq!(iter.next().unwrap(), "333");
    }

    #[test]
    fn test_big_sstable() {
        let mut buf = Vec::new();
        let mut builder =
            SSTableBuilder::with_bloom_filter(&mut buf, bloom_filter::BloomFilterBuilder::empty());

        let mut keys = Vec::new();
        for i in 0..1_000_000 {
            keys.push(format!("{}", i));
        }
        keys.sort();

        for key in &keys {
            let payload = format!("{}=={}", key, key);
            builder.write_ordered(key, payload).unwrap();
        }
        builder.finish().unwrap();

        let reader = SSTableReader::<&str>::from_bytes(&buf).unwrap();

        assert_eq!(reader.contains_duplicate_keys, false);
        assert_eq!(reader.index.keys.len(), 382);

        assert_eq!(reader.get(""), None);
        assert_eq!(reader.get("-1"), None);
        assert_eq!(reader.get("23456").unwrap(), "23456==23456");
        assert_eq!(reader.get("234567").unwrap(), "234567==234567");
        assert_eq!(reader.get("234567___"), None);
        assert_eq!(reader.get("567891").unwrap(), "567891==567891");
        assert_eq!(reader.get("999999").unwrap(), "999999==999999");
        assert_eq!(reader.get("9999999"), None);

        let mut iter = reader
            .iter_ek_at(Filter::from_spec("23456"))
            .map(|(_, v)| v);
        assert_eq!(iter.next().unwrap(), "23456==23456");
        assert_eq!(iter.next().unwrap(), "234560==234560");
        assert_eq!(iter.next().unwrap(), "234561==234561");
        assert_eq!(iter.next().unwrap(), "234562==234562");
        assert_eq!(iter.next().unwrap(), "234563==234563");
        assert_eq!(iter.next().unwrap(), "234564==234564");
        assert_eq!(iter.next().unwrap(), "234565==234565");
        assert_eq!(iter.next().unwrap(), "234566==234566");
        assert_eq!(iter.next().unwrap(), "234567==234567");
        assert_eq!(iter.next().unwrap(), "234568==234568");
        assert_eq!(iter.next().unwrap(), "234569==234569");
        assert_eq!(iter.next(), None);
    }
}

const BLOCK_SIZE: u64 = 65536;
const VERSION: u16 = 0;

mod index;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use itertools::{MinHeap, StreamingIterator, KV};
use primitive::{Primitive, Serializable};
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
        self.last_key = key.to_owned();

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

impl<T: Serializable + Default> SSTableReader<T> {
    pub fn new(file: std::fs::File) -> Result<Self> {
        let dtable = unsafe { mmap::MmapOptions::new().map(&file)? };

        Self::from_mmap(dtable)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut map = mmap::MmapMut::map_anon(bytes.len())?;
        map.copy_from_slice(bytes);
        Self::from_mmap(map.make_read_only()?)
    }

    pub fn from_mmap(dtable: mmap::Mmap) -> Result<Self> {
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

    pub fn from_filename(filename: &str) -> Result<Self> {
        let f = std::fs::File::open(filename)?;
        Self::new(f)
    }

    pub fn suggest_shards(&self, key_spec: &str, min_key: &str, max_key: &str) -> Vec<String> {
        index::suggest_shards(&self.index, key_spec, min_key, max_key)
    }

    pub fn get_shard_boundaries(&self, target_shard_count: usize) -> Vec<String> {
        index::get_shard_boundaries(&self.index, target_shard_count)
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

    fn get_offset_for_min_key(&self, key: &str) -> Result<usize> {
        let mut offset = match index::get_block_with_min_key(&self.index, key) {
            Some(block) => block.get_offset() as usize,
            None => return Ok(0),
        };

        loop {
            let (k, _, new_offset) = match self.read_at(offset)? {
                Some(x) => x,
                None => break,
            };

            if k >= key {
                break;
            }

            offset = new_offset;
        }

        Ok(offset)
    }
}

pub struct SpecdSSTableReader<'a, T: 'a> {
    reader: &'a SSTableReader<T>,
    key_spec: String,

    min_key: String,
    max_key: String,

    reached_end: bool,

    offset: usize,
}

impl<'a, T: Serializable + Default> SpecdSSTableReader<'a, T> {
    pub fn from_reader(reader: &'a SSTableReader<T>, key_spec: &str) -> SpecdSSTableReader<'a, T> {
        let mut specd_reader = SpecdSSTableReader {
            min_key: String::from(""),
            max_key: String::from(""),
            reader: reader,
            key_spec: key_spec.to_owned(),
            reached_end: false,
            offset: 0,
        };

        specd_reader.seek_to_start().unwrap();
        specd_reader
    }

    pub fn seek_to_start(&mut self) -> Result<()> {
        self.reached_end = false;
        let maybe_block = match self.min_key > self.key_spec {
            true => index::get_block_with_min_key(&self.reader.index, self.min_key.as_str()),
            false => index::get_block_with_keyspec(&self.reader.index, self.key_spec.as_str()),
        };

        self.offset = match maybe_block {
            Some(block) => block.get_offset() as usize,
            None => {
                self.reached_end = true;
                return Ok(());
            }
        };

        Ok(())
    }

    pub fn from_reader_with_scope(
        reader: &'a SSTableReader<T>,
        key_spec: &str,
        min_key: &str,
        max_key: &str,
    ) -> SpecdSSTableReader<'a, T> {
        let mut specd_reader = SpecdSSTableReader {
            reader: reader,
            key_spec: key_spec.to_owned(),
            reached_end: false,
            min_key: min_key.to_owned(),
            max_key: max_key.to_owned(),
            offset: 0,
        };

        specd_reader
    }

    // is_within_scope determines whether a key falls within the range specified by the specd
    // reader definition.
    fn is_within_scope(&self, key: &str) -> i8 {
        if key < self.min_key.as_str() {
            return -1;
        }

        // If the key doesn't start witht he prefix, we might be before or after the
        // prefix.
        if !key.starts_with(self.key_spec.as_str()) {
            return match key > self.key_spec.as_str() {
                true => 1,
                false => -1,
            };
        }

        if self.max_key != "" && key >= self.max_key.as_str() {
            return 1;
        }

        0
    }
}

impl<'a, T: Serializable + Default> Iterator for SpecdSSTableReader<'a, T> {
    type Item = (String, T);
    fn next(&mut self) -> Option<(String, T)> {
        if self.reached_end {
            return None;
        }

        loop {
            let (k, v, idx) = match self.reader.read_at(self.offset).unwrap() {
                Some(x) => x,
                None => return None,
            };

            self.offset = idx;

            if k < self.key_spec.as_str() {
                continue;
            }
            let mode = self.is_within_scope(k);
            return match mode {
                0 => Some((k.to_string(), T::from_bytes(v).unwrap())),
                1 => None,
                _ => continue,
            };
        }
    }
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

pub struct ShardedSSTableReader<T> {
    readers: Vec<SSTableReader<T>>,
    offsets: Vec<usize>,
    // A max key of "" means no max.
    max_key: String,
    heap: MinHeap<KV<KV<String, T>, usize>>,
    top: Option<KV<String, T>>,
}

impl<'a, T: Serializable + Default> ShardedSSTableReader<T> {
    pub fn from_readers(
        readers: Vec<SSTableReader<T>>,
        min_key: &str,
        max_key: String,
    ) -> ShardedSSTableReader<T> {
        let mut reader = ShardedSSTableReader {
            readers: readers,
            offsets: Vec::new(),
            max_key: max_key,
            heap: MinHeap::new(),
            top: None,
        };

        reader.seek(min_key);
        reader
    }

    pub fn seek(&mut self, min_key: &str) {
        // First, seek to the starting key in all the SSTables.
        self.offsets.clear();
        for r in &self.readers {
            self.offsets
                .push(r.get_offset_for_min_key(min_key).unwrap());
        }

        // Next, we'll construct a heap using our keys. This will allow us to efficiently
        // insert while keeping a sorted list.
        self.heap.clear();
        for index in 0..self.readers.len() {
            let (k, v, new_offset) = match self.readers[index].read_at(self.offsets[index]).unwrap()
            {
                Some(x) => x,
                None => continue,
            };

            self.offsets[index] = new_offset;

            let value = T::from_bytes(v).unwrap();
            self.heap.push(KV::new(KV::new(k.to_owned(), value), index));
        }
    }

    pub fn from_filenames(filenames: &[String], min_key: &str, max_key: String) -> Result<Self> {
        let (successes, failures): (Vec<_>, Vec<_>) = filenames
            .iter()
            .map(|f| SSTableReader::from_filename(f))
            .partition(Result::is_ok);

        if failures.len() != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "failed to open sharded sstable",
            ));
        }

        let readers = successes.into_iter().map(|r| r.unwrap()).collect();

        let mut reader = ShardedSSTableReader {
            offsets: Vec::new(),
            readers: readers,
            max_key: max_key,
            heap: MinHeap::new(),
            top: None,
        };
        reader.seek(min_key);
        Ok(reader)
    }

    pub fn from_filename(filename: &str, min_key: &str, max_key: String) -> Result<Self> {
        let filenames = shard_lib::unshard(filename);
        Self::from_filenames(&filenames, min_key, max_key)
    }

    pub fn get_shard_boundaries(&self, target_shard_count: usize) -> Vec<String> {
        let mut output = Vec::new();
        for shard in &self.readers {
            output.append(&mut shard.get_shard_boundaries(target_shard_count));
        }

        shard_lib::compact_shards(output, target_shard_count)
    }

    pub fn next(&mut self) -> Option<(String, T)> {
        let (kv, index) = match self.heap.pop() {
            Some(KV(k, v)) => (k, v),
            None => return None,
        };

        match self.readers[index].read_at(self.offsets[index]).unwrap() {
            Some((k, v, new_offset)) => {
                if self.max_key.as_str() == "" || k < self.max_key.as_str() {
                    let value = T::from_bytes(v).unwrap();
                    self.heap.push(KV::new(KV::new(k.to_owned(), value), index));
                }

                self.offsets[index] = new_offset;
            }
            None => (),
        };

        if self.max_key.as_str() == "" || kv.key() < &self.max_key {
            let KV(k, v) = kv;
            Some((k, v))
        } else {
            None
        }
    }
}

impl<'a, T: Serializable + Default> Iterator for ShardedSSTableReader<T> {
    type Item = (String, T);
    fn next(&mut self) -> Option<(String, T)> {
        return self.next();
    }
}

impl<T> StreamingIterator for ShardedSSTableReader<T>
where
    T: Serializable + Default,
{
    type Item = KV<String, T>;
    fn next(&mut self) -> Option<&Self::Item> {
        let (top, idx) = match self.heap.pop() {
            Some(KV(kv, idx)) => (Some(kv), Some(idx)),
            None => (None, None),
        };

        if let Some(index) = idx {
            match self.readers[index].read_at(self.offsets[index]).unwrap() {
                Some((k, v, new_offset)) => {
                    if self.max_key.as_str() == "" || k < self.max_key.as_str() {
                        let value = T::from_bytes(v).unwrap();
                        self.heap
                            .push(KV::new(KV::new(k.to_string(), value), index));
                    }

                    self.offsets[index] = new_offset;
                }
                None => (),
            };
        }

        self.top = top;

        match &self.top {
            Some(kv) => Some(kv),
            None => None,
        }
    }

    fn peek(&mut self) -> Option<&Self::Item> {
        match self.heap.peek() {
            Some(o) => Some(o.key()),
            None => None,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ReshardTask {
    Split(String, Vec<String>),
    Copy(String, String),
    Merge(Vec<String>, String),
}

pub fn plan_reshard(sources: &[String], sinks: &[String]) -> Vec<ReshardTask> {
    let reversed = sinks.len() > sources.len();
    let (from, to) = if reversed {
        (sinks, sources)
    } else {
        (sources, sinks)
    };

    let mut links = Vec::new();
    for _ in to {
        links.push(Vec::new());
    }

    for (index, f) in from.iter().enumerate() {
        links[index % to.len()].push(f.to_string());
    }

    let mut plans = Vec::new();
    for (x, ys) in to.iter().zip(links.into_iter()) {
        if reversed {
            if ys.len() == 1 {
                plans.push(ReshardTask::Copy(x.into(), ys[0].clone()));
            } else {
                plans.push(ReshardTask::Split(x.into(), ys));
            }
        } else {
            if ys.len() == 1 {
                plans.push(ReshardTask::Copy(ys[0].clone(), x.into()));
            } else {
                plans.push(ReshardTask::Merge(ys, x.into()));
            }
        }
    }
    plans
}

pub fn execute_reshard_task(task: ReshardTask) {
    match task {
        ReshardTask::Copy(from, to) => {
            std::fs::copy(from, to).unwrap();
        }
        ReshardTask::Split(from, to) => {
            let reader = SSTableReader::<Primitive<Vec<u8>>>::from_filename(&from).unwrap();
            let mut boundaries = reader.get_shard_boundaries(to.len());
            let mut source = reader.peekable();
            boundaries.push(String::new());
            for (boundary, to_filename) in boundaries.iter().zip(to.iter()) {
                let f = std::fs::File::create(to_filename).unwrap();
                let mut w = std::io::BufWriter::new(f);
                let mut builder = SSTableBuilder::<Primitive<Vec<u8>>, _>::new(&mut w);

                loop {
                    if let Some((k, _)) = source.peek() {
                        if !boundary.is_empty() && k > boundary {
                            break;
                        }
                    } else {
                        break;
                    }
                    let (k, v) = source.next().unwrap();
                    builder.write_ordered(&k, v).unwrap();
                }

                builder.finish().unwrap();
            }
        }
        ReshardTask::Merge(from, to) => {
            let reader = ShardedSSTableReader::<Primitive<Vec<u8>>>::from_filenames(
                &from,
                "",
                String::new(),
            )
            .unwrap();
            let f = std::fs::File::open(to).unwrap();
            let mut w = std::io::BufWriter::new(f);
            let mut builder = SSTableBuilder::new(&mut w);
            for (k, v) in reader {
                builder.write_ordered(&k, v).unwrap();
            }
            builder.finish().unwrap();
        }
    }
}

pub fn reshard(from: &[String], to: &[String]) {
    for task in plan_reshard(from, to) {
        execute_reshard_task(task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitive::Primitive;
    use std::io::Seek;

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

    #[test]
    fn test_reshard_planning() {
        let from = vec!["a.sstable".into(), "b.sstable".into()];
        let to = vec![
            "x0.sstable".into(),
            "x1.sstable".into(),
            "x2.sstable".into(),
        ];
        let expected = vec![
            ReshardTask::Split(
                "a.sstable".into(),
                vec!["x0.sstable".into(), "x2.sstable".into()],
            ),
            ReshardTask::Copy("b.sstable".into(), "x1.sstable".into()),
        ];
        let plans = plan_reshard(&from, &to);
        assert_eq!(plans, expected);
    }

    #[test]
    fn test_reshard_planning_2() {
        let from = vec!["a.sstable".into(), "b.sstable".into(), "c.sstable".into()];
        let to = vec!["x.sstable".into()];
        let expected = vec![ReshardTask::Merge(
            vec!["a.sstable".into(), "b.sstable".into(), "c.sstable".into()],
            "x.sstable".into(),
        )];
        let plans = plan_reshard(&from, &to);
        assert_eq!(plans, expected);
    }

    #[test]
    fn serialize_i64() {
        let value: Primitive<i64> = Primitive(5);
        let mut k = std::io::Cursor::new(Vec::new());
        {
            value.write(&mut k).unwrap();
        }
        let mut z: Primitive<i64> = Primitive(9);
        {
            z.read_from_bytes(&k.into_inner()).unwrap();
        }
        assert_eq!(z, 5);
    }

    #[test]
    fn serialize_proto() {
        let mut value = sstable_proto_rust::KeyEntry::new();
        value.set_key(String::from("hello world"));
        value.set_offset(1234);
        let mut k = std::io::Cursor::new(Vec::new());
        {
            value.write(&mut k).unwrap();
        }
        let mut output = sstable_proto_rust::KeyEntry::new();
        {
            output.read_from_bytes(&k.into_inner()).unwrap();
        }
        assert_eq!(output.get_key(), "hello world");
        assert_eq!(output.get_offset(), 1234);
    }
    #[test]
    #[should_panic]
    fn construct_sstable_builder_backwards() {
        let mut k = std::io::Cursor::new(Vec::new());
        let mut t = SSTableBuilder::<Primitive<f64>, _>::new(&mut k);
        t.write_ordered("camel", Primitive(1.0234)).unwrap();
        // This write is out of order, which should be caught by an assertion.
        t.write_ordered("baboon", Primitive(0.222)).unwrap();
    }

    #[test]
    fn read_next_key_on_constructed_sstable() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d);
            t.write_ordered("hello", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let mut r = SSTableReader::<Primitive<i64>>::from_bytes(&d.into_inner()).unwrap();
            let entry = r.next().unwrap();
            assert_eq!(&entry.0, "hello");
        }
    }

    #[test]
    fn read_constructed_sstable_with_iter() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d);
            t.write_ordered("hello", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let r = SSTableReader::<Primitive<i64>>::from_bytes(&d.into_inner()).unwrap();
            assert_eq!(
                r.collect::<Vec<_>>(),
                vec![(String::from("hello"), Primitive(5))]
            )
        }
    }

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
        let reader = SSTableReader::<Primitive<i64>>::from_bytes(&f.into_inner()).unwrap();
        let mut x = 0;
        for (strx, intx) in reader {
            assert_eq!(format!("{:9}", x).as_str(), strx);
            assert_eq!(intx, x);
            x += 1;
        }
    }
    #[test]
    fn find_a_key() {
        let mut f = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut f);
            t.write_ordered("a special key", Primitive(500)).unwrap();
            t.write_ordered("special key", Primitive(1234)).unwrap();
            t.write_ordered("zzz key", Primitive(400)).unwrap();
            t.finish().unwrap();
        }
        let mut reader = SSTableReader::<Primitive<i64>>::from_bytes(&f.into_inner()).unwrap();
        assert_eq!(reader.get("a special key").unwrap(), Some(Primitive(500)));
        assert_eq!(reader.get("nonexistent key").unwrap(), None);
        assert_eq!(reader.get("special key").unwrap(), Some(Primitive(1234)));
        assert_eq!(reader.get("zzz key").unwrap(), Some(Primitive(400)));
    }
    fn keyentry(key: &str, offset: u64) -> sstable_proto_rust::KeyEntry {
        let mut k = sstable_proto_rust::KeyEntry::new();
        k.set_key(key.to_owned());
        k.set_offset(offset);
        k
    }
    #[test]
    fn find_a_block() {
        let mut t = sstable_proto_rust::Index::new();
        assert_eq!(index::get_block(&t, "asdf"), None);
        t.set_pointers(protobuf::RepeatedField::from_vec(vec![keyentry(
            "bloop", 123,
        )]));
        assert_eq!(index::get_block(&t, "asdf"), None);
        assert_eq!(index::get_block(&t, "b"), None);
        assert_eq!(index::get_block(&t, "bloop"), Some(keyentry("bloop", 123)));
        assert_eq!(index::get_block(&t, "blooq"), Some(keyentry("bloop", 123)));
    }
    #[test]
    fn find_a_key_with_many_keys() {
        let mut f = std::io::Cursor::new(Vec::new());
        let key = "very long key very very long key extremely long key";
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut f);
            // Write 10k even numbers.
            for i in 0..10000 {
                t.write_ordered(format!("{}-->{:9}", key, i * 2).as_str(), Primitive(i * 2))
                    .unwrap();
            }
            t.finish().unwrap();
        }
        let mut reader = SSTableReader::<Primitive<i64>>::from_bytes(&f.into_inner()).unwrap();
        assert_eq!(
            reader
                .get(format!("{}-->{:9}", key, 3201).as_str())
                .unwrap(),
            None
        );
        assert_eq!(
            reader.get(format!("{}-->{:9}", key, 0).as_str()).unwrap(),
            Some(Primitive(0))
        );
        assert_eq!(
            reader
                .get(format!("{}-->{:9}", key, 9000).as_str())
                .unwrap(),
            Some(Primitive(9000))
        );
    }
    #[test]
    fn read_with_nonexistent_key_spec() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d);
            t.write_ordered("allow", Primitive(5)).unwrap();
            t.write_ordered("bellow", Primitive(5)).unwrap();
            t.write_ordered("wallow", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let mut r = SSTableReader::<Primitive<i64>>::from_bytes(&d.into_inner()).unwrap();
            let specd_reader = SpecdSSTableReader::from_reader(&mut r, "hello-");
            assert_eq!(
                specd_reader.map(|(k, _)| k).collect::<Vec<String>>(),
                Vec::<String>::new()
            );
        }
    }
    #[test]
    fn read_with_key_spec() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d);
            t.write_ordered("allow", Primitive(5)).unwrap();
            t.write_ordered("bellow", Primitive(5)).unwrap();
            t.write_ordered("hello-1", Primitive(5)).unwrap();
            t.write_ordered("hello-2", Primitive(5)).unwrap();
            t.write_ordered("hello-3", Primitive(5)).unwrap();
            t.write_ordered("wallow", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let mut r = SSTableReader::<Primitive<i64>>::from_bytes(&d.into_inner()).unwrap();
            let specd_reader = SpecdSSTableReader::from_reader(&mut r, "hello-");
            assert_eq!(
                specd_reader.map(|(k, _)| k).collect::<Vec<_>>(),
                vec!["hello-1", "hello-2", "hello-3"]
            );
        }
    }
    #[test]
    fn test_within_scope() {
        // Construct an empty SSTable.
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d);
            t.finish().unwrap();
        }
        let mut r = SSTableReader::<Primitive<i64>>::from_bytes(&d.into_inner()).unwrap();
        let specd_reader =
            SpecdSSTableReader::from_reader_with_scope(&mut r, "hello", "hello-co", "hello-te");
        // Before scope
        assert_eq!(specd_reader.is_within_scope(""), -1);
        assert_eq!(specd_reader.is_within_scope("hello-apple"), -1);
        // Within scope
        // By convention, the min key is included in the scope.
        assert_eq!(specd_reader.is_within_scope("hello-co"), 0);
        assert_eq!(specd_reader.is_within_scope("hello-colin"), 0);
        assert_eq!(specd_reader.is_within_scope("hello-darling"), 0);
        assert_eq!(specd_reader.is_within_scope("hello-tambourine"), 0);
        // Beyond scope
        // By convention, the max key is excluded from the scope.
        assert_eq!(specd_reader.is_within_scope("hello-te"), 1);
        assert_eq!(specd_reader.is_within_scope("hello-test"), 1);
        assert_eq!(specd_reader.is_within_scope("world"), 1);
    }
    #[test]
    fn get_block_with_min_key() {
        let pointers = vec![
            keyentry("aaaa", 0),
            keyentry("bbbb", 1),
            keyentry("cccc", 2),
            keyentry("dddd", 3),
        ];
        let mut index = sstable_proto_rust::Index::new();
        index.set_pointers(protobuf::RepeatedField::from_vec(pointers));
        assert_eq!(
            index::get_block_with_min_key(&index, "argument"),
            Some(keyentry("aaaa", 0))
        );
        assert_eq!(
            index::get_block_with_min_key(&index, "dog"),
            Some(keyentry("dddd", 3))
        );
        assert_eq!(
            index::get_block_with_min_key(&index, "000"),
            Some(keyentry("aaaa", 0))
        );
    }
    #[test]
    fn test_jump_to_min_key() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d);
            t.write_ordered("allow", Primitive(5)).unwrap();
            t.write_ordered("bellow", Primitive(5)).unwrap();
            t.write_ordered("hello-1", Primitive(5)).unwrap();
            t.write_ordered("hello-2", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut r = SSTableReader::<Primitive<i64>>::from_bytes(&d.into_inner()).unwrap();
        let offset = r.get_offset_for_min_key("c").unwrap();

        let (k, v, new_offset) = r.read_at(offset).unwrap().unwrap();
        assert_eq!(k, "hello-1");

        let (k, v, _) = r.read_at(new_offset).unwrap().unwrap();
        assert_eq!(k, "hello-2");
    }

    #[test]
    fn test_sharded_read() {
        let mut d1 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d1);
            t.write_ordered("aardvark", Primitive(5)).unwrap();
            t.write_ordered("bee", Primitive(5)).unwrap();
            t.write_ordered("cat", Primitive(5)).unwrap();
            t.write_ordered("dog", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d1.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut d2 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d2);
            t.write_ordered("apple", Primitive(5)).unwrap();
            t.write_ordered("banana", Primitive(5)).unwrap();
            t.write_ordered("cantaloupe", Primitive(5)).unwrap();
            t.write_ordered("durian", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d2.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let r1 = SSTableReader::<Primitive<i64>>::from_bytes(&d1.into_inner()).unwrap();
            let r2 = SSTableReader::<Primitive<i64>>::from_bytes(&d2.into_inner()).unwrap();
            let s = ShardedSSTableReader::<Primitive<i64>>::from_readers(
                vec![r1, r2],
                "c",
                String::from(""),
            );
            assert_eq!(
                s.map(|(k, _)| k).collect::<Vec<_>>(),
                vec!["cantaloupe", "cat", "dog", "durian"]
            );
        }
    }
    #[test]
    fn test_sharded_read_with_max_key() {
        let mut d1 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d1);
            t.write_ordered("aardvark", Primitive(5)).unwrap();
            t.write_ordered("bee", Primitive(5)).unwrap();
            t.write_ordered("cat", Primitive(5)).unwrap();
            t.write_ordered("dog", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        let mut d2 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d2);
            t.write_ordered("apple", Primitive(5)).unwrap();
            t.write_ordered("banana", Primitive(5)).unwrap();
            t.write_ordered("cantaloupe", Primitive(5)).unwrap();
            t.write_ordered("durian", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        {
            let r1 = SSTableReader::<Primitive<i64>>::from_bytes(&d1.into_inner()).unwrap();
            let r2 = SSTableReader::<Primitive<i64>>::from_bytes(&d2.into_inner()).unwrap();
            let s = ShardedSSTableReader::<Primitive<i64>>::from_readers(
                vec![r1, r2],
                "c",
                String::from("cucumber"),
            );
            assert_eq!(
                s.map(|(k, v)| k).collect::<Vec<_>>(),
                vec!["cantaloupe", "cat"]
            );
        }
    }
    #[test]
    fn test_shard_suggestion() {
        let pointers = vec![
            keyentry("0000", 0),
            keyentry("aaaa", 1),
            keyentry("bbbb", 2),
            keyentry("cccc", 3),
            keyentry("dddd", 4),
            keyentry("zzzz", 5),
        ];
        let mut index = sstable_proto_rust::Index::new();
        index.set_pointers(protobuf::RepeatedField::from_vec(pointers));
        let expected = vec![
            String::from("aaaa"),
            String::from("bbbb"),
            String::from("cccc"),
            String::from("dddd"),
        ];
        assert_eq!(index::suggest_shards(&index, "", "a", "z"), expected);
    }
    #[test]
    fn get_block_with_keyspec() {
        let pointers = vec![
            keyentry("animals_cat", 0),
            keyentry("animals_dog", 1),
            keyentry("animals_yak", 2),
            keyentry("people_colin", 3),
            keyentry("people_drew", 4),
            keyentry("people_yang", 5),
            keyentry("places_dubai", 6),
            keyentry("places_london", 7),
            keyentry("places_toronto", 8),
            keyentry("things_pineapple", 9),
        ];
        let mut index = sstable_proto_rust::Index::new();
        index.set_pointers(protobuf::RepeatedField::from_vec(pointers));
        assert_eq!(
            index::get_block_with_keyspec(&index, "animals_"),
            Some(keyentry("animals_cat", 0))
        );
        assert_eq!(
            index::get_block_with_keyspec(&index, "people_"),
            Some(keyentry("animals_yak", 2))
        );
        assert_eq!(
            index::get_block_with_keyspec(&index, "places_"),
            Some(keyentry("people_yang", 5))
        );
        assert_eq!(
            index::get_block_with_keyspec(&index, "things_"),
            Some(keyentry("places_toronto", 8))
        );
    }
    #[test]
    fn test_shard_suggestion_with_keyspec() {
        let pointers = vec![
            keyentry("people_colin", 3),
            keyentry("people_drew", 4),
            keyentry("people_yang", 5),
            keyentry("places_dubai", 0),
            keyentry("places_london", 1),
            keyentry("places_toronto", 2),
            keyentry("things_pineapple", 5),
        ];
        let mut index = sstable_proto_rust::Index::new();
        index.set_pointers(protobuf::RepeatedField::from_vec(pointers));
        let expected = vec![
            String::from("people_colin"),
            String::from("people_drew"),
            String::from("people_yang"),
        ];
        assert_eq!(index::suggest_shards(&index, "people_", "", ""), expected);
    }
    #[test]
    fn test_shard_suggestion_with_min_max() {
        let pointers = vec![
            keyentry("people_colin", 3),
            keyentry("people_drew", 4),
            keyentry("people_yang", 5),
            keyentry("places_dubai", 0),
            keyentry("places_london", 1),
            keyentry("places_toronto", 2),
            keyentry("things_pineapple", 5),
        ];
        let mut index = sstable_proto_rust::Index::new();
        index.set_pointers(protobuf::RepeatedField::from_vec(pointers));
        let expected = vec![
            String::from("people_colin"),
            String::from("people_drew"),
            String::from("people_yang"),
            String::from("places_dubai"),
        ];
        assert_eq!(
            index::suggest_shards(&index, "", "people_c", "places_e"),
            expected
        );
    }
    #[test]
    fn test_shard_boundaries() {
        let pointers = vec![
            keyentry("people_colin", 3),
            keyentry("people_drew", 4),
            keyentry("people_yang", 5),
            keyentry("places_dubai", 0),
            keyentry("places_london", 1),
            keyentry("places_toronto", 2),
            keyentry("things_pineapple", 5),
        ];
        let mut index = sstable_proto_rust::Index::new();
        index.set_pointers(protobuf::RepeatedField::from_vec(pointers));
        let expected = vec![String::from("people_yang"), String::from("places_london")];
        assert_eq!(index::get_shard_boundaries(&index, 3), expected);
    }
    #[test]
    fn test_shard_boundaries_2() {
        let pointers = vec![
            keyentry("people_colin", 3),
            keyentry("people_drew", 4),
            keyentry("people_yang", 5),
            keyentry("places_dubai", 0),
            keyentry("places_london", 1),
            keyentry("places_toronto", 2),
            keyentry("things_pineapple", 5),
        ];
        let mut index = sstable_proto_rust::Index::new();
        index.set_pointers(protobuf::RepeatedField::from_vec(pointers));
        let expected = vec![
            String::from("people_drew"),
            String::from("people_yang"),
            String::from("places_london"),
            String::from("places_toronto"),
        ];
        assert_eq!(index::get_shard_boundaries(&index, 5), expected);
    }
    #[test]
    fn test_stream_iterator() {
        let mut d1 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d1);
            t.write_ordered("a", Primitive(5)).unwrap();
            t.write_ordered("b", Primitive(5)).unwrap();
            t.write_ordered("cat", Primitive(5)).unwrap();
            t.write_ordered("d", Primitive(5)).unwrap();
            t.write_ordered("e", Primitive(5)).unwrap();
            t.write_ordered("f", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        let mut d2 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d2);
            t.write_ordered("apple", Primitive(5)).unwrap();
            t.write_ordered("banana", Primitive(5)).unwrap();
            t.write_ordered("cantaloupe", Primitive(5)).unwrap();
            t.write_ordered("durian", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        let r1 = SSTableReader::<Primitive<i64>>::from_bytes(&d1.into_inner()).unwrap();
        let r2 = SSTableReader::<Primitive<i64>>::from_bytes(&d2.into_inner()).unwrap();
        let mut s = ShardedSSTableReader::<Primitive<i64>>::from_readers(
            vec![r1, r2],
            "c",
            String::from("cucumber"),
        );
        let mut iter: &mut dyn StreamingIterator<Item = KV<String, Primitive<i64>>> = &mut s;
        assert_eq!(
            iter.next(),
            Some(&KV::new("cantaloupe".into(), Primitive(5)))
        );
        assert_eq!(iter.next(), Some(&KV::new("cat".into(), Primitive(5))));
    }
    #[test]
    fn test_stream_iterator_2() {
        let mut d1 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d1);
            t.write_ordered("a", Primitive(5)).unwrap();
            t.write_ordered("b", Primitive(5)).unwrap();
            t.write_ordered("cat", Primitive(5)).unwrap();
            t.write_ordered("d", Primitive(5)).unwrap();
            t.write_ordered("e", Primitive(5)).unwrap();
            t.write_ordered("f", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        let mut d2 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>, _>::new(&mut d2);
            t.write_ordered("apple", Primitive(5)).unwrap();
            t.write_ordered("banana", Primitive(5)).unwrap();
            t.write_ordered("cantaloupe", Primitive(5)).unwrap();
            t.write_ordered("durian", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        let r1 = SSTableReader::<Primitive<i64>>::from_bytes(&d1.into_inner()).unwrap();
        let r2 = SSTableReader::<Primitive<i64>>::from_bytes(&d2.into_inner()).unwrap();
        let mut s =
            ShardedSSTableReader::<Primitive<i64>>::from_readers(vec![r1, r2], "a", String::new());
        let mut iter: &mut dyn StreamingIterator<Item = KV<String, Primitive<i64>>> = &mut s;
        assert_eq!(iter.peek(), Some(&KV::new("a".into(), Primitive(5))));
        assert_eq!(iter.next(), Some(&KV::new("a".into(), Primitive(5))));
        assert_eq!(iter.peek(), Some(&KV::new("apple".into(), Primitive(5))));
        assert_eq!(iter.next(), Some(&KV::new("apple".into(), Primitive(5))));
    }
}

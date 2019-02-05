/*
 * sstable.rs
 *
 * This library implements methods for interacting with sstables. An SSTable is a key-value table
 * which is immutable, and all the keys are ordered. There are two files that compose an SSTable,
 * the keys file and the database file.
 */

extern crate sstable_proto_rust;

extern crate byteorder;
extern crate primitive;
extern crate protobuf;
pub use primitive::{Primitive, Serializable};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use protobuf::Message;
use std::collections::BTreeMap;
use std::default::Default;
use std::fs::File;
use std::io;
use std::io::Read;

use std::io::Seek;

// The defualt block size is 64 kilobytes.
const BLOCK_SIZE: u64 = 64000;

pub struct SSTableBuilder<'a, T: Serializable> {
    // The index contains a list of pointers into the dtable which we can use
    // to quickly look up locations in the file. It's written to the end of the dtable,
    // and loaded into memory when the sstable is read.
    index: sstable_proto_rust::Index,

    // The dtable is where the data is actually stored (e.g. the file on disk).
    dtable: &'a mut std::io::Write,

    // dtable_offset tells us the byte offset where the start of the current struct in the dtable
    // is. It gets incremented as we write new records.
    dtable_offset: u64,
    block_offset: u64,
    at_start_of_block: bool,

    // We have to explicitly state that the struct uses the type T, or else the rust compiler will
    // get confused. This is a zero-size type to help the compiler infer the usage of T.
    data_type: std::marker::PhantomData<T>,

    // We store the last key that was written to the SSTable so we can ensure that it is written in
    // order. If you write the SSTable out of order, it won't work correctly.
    last_key: String,

    // Once the "finish" step has been completed, we should not write any additional keys. This
    // flag is used to make sure we don't do that.
    finished: bool,
}

pub struct SSTableReader<T> {
    index: sstable_proto_rust::Index,
    dtable: Box<SeekableRead>,

    // We have to explicitly state that the struct uses the type T, or else the rust compiler will
    // get confused. This is a zero-size type to help the compiler infer the usage of T.
    data_type: std::marker::PhantomData<T>,

    // dtable_offset tells us how far we've read in the dtable.
    dtable_offset: u64,

    // The index_offset is where the data ends and the index starts. It's written to the last eight
    // bytes of the file.
    index_offset: u64,
}

pub struct SpecdSSTableReader<'a, T: 'a> {
    reader: &'a mut SSTableReader<T>,
    key_spec: String,

    min_key: String,
    max_key: String,

    reached_end: bool,
}

pub struct ShardedSSTableReader<T> {
    readers: Vec<SSTableReader<T>>,
    // A max key of "" means no max.
    max_key: String,
    map: BTreeMap<String, (usize, Vec<u8>)>,
    reached_end: bool,
}

// We write SSTables in a single pass across the disk, but reading
// requires seeking to get to the index table. So we need both Seek
// and Read traits.
pub trait SeekableRead: io::Seek + io::Read + Send + Sync {}
impl<T: io::Seek + io::Read + Send + Sync> SeekableRead for T {}

impl<'a, T: Serializable + Default> SSTableBuilder<'a, T> {
    pub fn new(dtable: &'a mut std::io::Write) -> SSTableBuilder<'a, T> {
        SSTableBuilder {
            index: sstable_proto_rust::Index::new(),
            dtable: dtable,
            dtable_offset: 0,
            block_offset: 0,
            at_start_of_block: true,
            data_type: std::marker::PhantomData,
            last_key: String::new(),
            finished: false,
        }
    }

    pub fn write_ordered(&mut self, key: &str, value: T) -> std::io::Result<()> {
        // Prepare the data entry proto.
        let mut buffer = vec![];
        value.write(&mut buffer)?;

        self.write_raw(key, &buffer)
    }

    // You can use this to write raw binary data into the sstable, if you don't
    // want to specify a serializable type. This is helpful if you don't want to
    // deserialize the data before writing it, and you already have it in binary
    // form.
    pub fn write_raw(&mut self, key: &str, value: &[u8]) -> std::io::Result<()> {
        assert!(
            self.dtable_offset == 0 || key >= self.last_key.as_str(),
            format!(
                "Keys must be written in order to the SSTable!\n `{}` (written) < `{}` (previous)",
                key, self.last_key
            )
        );
        assert!(
            !self.finished,
            "Attempted to write to the sstable after finish()"
        );

        // If we're at the start of the block, we'll need to write a record into
        // the index.
        if self.at_start_of_block {
            self.at_start_of_block = false;
            let mut key_entry = sstable_proto_rust::KeyEntry::new();
            key_entry.set_key(String::from(key));
            key_entry.set_offset(self.dtable_offset);
            // Begin by preparing the key entry.
            let key_entries = self.index.mut_pointers();
            key_entries.push(key_entry);
        }

        self.last_key = key.to_owned();

        let mut data_entry = sstable_proto_rust::DataEntry::new();
        data_entry.set_key(String::from(key));
        data_entry.set_value(value.to_owned());

        // Write the length, then the proto. The length includes the 4 size bytes.
        let size = data_entry.compute_size();
        self.dtable.write_u32::<LittleEndian>(size)?;
        data_entry.write_to_writer(self.dtable).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unexpectedly unable to write data record: {}", e),
            )
        })?;

        // Update the internal offsets.
        self.dtable_offset += 4 + size as u64;
        self.block_offset += 4 + size as u64;

        // If we've crossed a block boundary, we'll write an index for the next entry.
        if self.block_offset > BLOCK_SIZE {
            self.block_offset = 0;
            self.at_start_of_block = true;
        }

        Ok(())
    }

    // When you're done writing, you need to call finish() in order to write the index to the end
    // of the sstable.
    pub fn finish(&mut self) -> io::Result<()> {
        self.index.write_to_writer(self.dtable).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unexpectedly unable to write index during finish(): {}", e),
            )
        })?;

        // Now that we've written the index, we must finish by writing the index where the
        // index was written to with respect to  the start of the file. We'll also write the
        // size of the index. This always consumes the last 16 bytes of the file.
        self.dtable.write_u64::<LittleEndian>(self.dtable_offset)?;
        self.dtable
            .write_u64::<LittleEndian>(self.index.compute_size() as u64)?;
        self.finished = true;
        Ok(())
    }

    // This function merges a series of sstables into one.
    pub fn from_sstables(
        dtable: &'a mut std::io::Write,
        readers: &mut [SSTableReader<T>],
    ) -> io::Result<()> {
        let mut builder = SSTableBuilder::<T>::new(dtable);
        let mut map = BTreeMap::<String, (usize, Vec<u8>)>::new();

        // First, we'll construct a BTreeMap using our keys. This will allow us to efficiently
        // insert while keeping a sorted list.
        for index in 0..readers.len() {
            let mut data = match readers[index].read_next_key() {
                Ok(Some(x)) => x,
                Ok(None) => continue,
                Err(e) => return Err(e),
            };

            map.insert(
                data.get_key().to_owned(),
                (index as usize, data.take_value()),
            );
        }

        loop {
            let (key, index) = {
                // Now we'll pop off the lowest key and write it into the output sstable. Then we'll
                // refresh the key by reading from the sstable the key originated from.
                let (key, &(index, ref value)) = match map.iter().next() {
                    Some(x) => x,
                    None => break,
                };

                builder.write_raw(key, &value)?;

                (key.to_owned(), index)
            };
            map.remove(key.as_str()).unwrap();

            let mut data = match readers[index].read_next_key() {
                Ok(Some(x)) => x,
                Ok(None) => continue,
                Err(e) => return Err(e),
            };

            map.insert(
                data.get_key().to_owned(),
                (index as usize, data.take_value()),
            );
        }

        builder.finish()?;
        Ok(())
    }
}

impl<T: Serializable + Default> SSTableReader<T> {
    pub fn new(mut dtable: Box<SeekableRead>) -> io::Result<SSTableReader<T>> {
        // First, seek directly to the end, which is where we store the location and size of the
        // index table.
        dtable.seek(io::SeekFrom::End(-16))?;
        let index_offset = dtable.read_u64::<LittleEndian>()?;
        let index_size = dtable.read_u64::<LittleEndian>()?;

        // Now let's jump to the index table and read that...
        dtable.seek(io::SeekFrom::Start(index_offset))?;
        let index = protobuf::parse_from_reader::<sstable_proto_rust::Index>(
            &mut (&mut dtable).take(index_size),
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unable to read sstable index: {}", e),
            )
        })?;

        // Reset the cursor to the start of the file, where the data begins. We're ready for
        // reading.
        dtable.seek(io::SeekFrom::Start(0))?;

        Ok(SSTableReader {
            dtable: dtable,
            index: index,
            data_type: std::marker::PhantomData,
            dtable_offset: 0,
            index_offset: index_offset,
        })
    }

    pub fn from_filename(filename: &str) -> io::Result<SSTableReader<T>> {
        let f = File::open(filename).unwrap();
        SSTableReader::new(Box::new(f))
    }

    fn seek_to_min_key(&mut self, key: &str) -> io::Result<()> {
        let starting_offset = match index::get_block_with_min_key(&self.index, key) {
            Some(block) => block.get_offset(),
            None => return Ok(()),
        };
        self.seek(starting_offset)?;

        let mut offset_to_reset_to;
        loop {
            offset_to_reset_to = self.dtable_offset;
            let current_key = match self.read_key()? {
                Some(x) => x,
                None => return Ok(()),
            };
            if current_key.as_str() > key {
                break;
            }
        }

        self.seek(offset_to_reset_to)?;

        Ok(())
    }

    // Rather than calling seek directly on the dtable, it's better to use self.seek, because
    // that way we can always keep track of the offset correctly.
    fn seek(&mut self, offset: u64) -> io::Result<()> {
        self.dtable.seek(io::SeekFrom::Start(offset))?;
        self.dtable_offset = offset;
        Ok(())
    }

    // get looks through the index and tries to find out whether a key is present in the sstable.
    // It may read a block, if it thinks the key may be in there.
    pub fn get(&mut self, key: &str) -> io::Result<Option<T>> {
        // First, conduct a binary search on the keys in the index to find the block to read. Then,
        // seek to that block, and read until the block is over.
        let offset = match index::get_block(&self.index, key) {
            Some(block) => block.get_offset(),
            None => return Ok(None),
        };

        self.seek(offset)?;

        loop {
            let key_entry = match self.read_next_key() {
                Ok(Some(de)) => de,
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            };
            if key_entry.get_key() > key {
                // We already passed the key, so it doesn't exist.
                return Ok(None);
            } else if key == key_entry.get_key() {
                let mut value = T::default();
                value.read_from_bytes(&key_entry.get_value())?;
                return Ok(Some(value));
            }
        }
    }
}

impl<'a, T: Serializable + Default> SpecdSSTableReader<'a, T> {
    pub fn from_reader(
        reader: &'a mut SSTableReader<T>,
        key_spec: &str,
    ) -> SpecdSSTableReader<'a, T> {
        let mut specd_reader = SpecdSSTableReader {
            min_key: String::from(""),
            max_key: String::from(""),
            reader: reader,
            key_spec: key_spec.to_owned(),
            reached_end: false,
        };

        specd_reader.seek_to_start().unwrap();
        specd_reader
    }

    pub fn from_reader_with_scope(
        reader: &'a mut SSTableReader<T>,
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
        };

        specd_reader.seek_to_start().unwrap();
        specd_reader
    }

    pub fn seek_to_start(&mut self) -> io::Result<()> {
        self.reached_end = false;
        let maybe_block = match self.min_key > self.key_spec {
            true => index::get_block_with_min_key(&self.reader.index, self.min_key.as_str()),
            false => index::get_block_with_keyspec(&self.reader.index, self.key_spec.as_str()),
        };

        let offset = match maybe_block {
            Some(block) => block.get_offset(),
            None => {
                self.reached_end = true;
                return Ok(());
            }
        };

        self.reader.seek(offset)
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

impl<'a, T: Serializable + Default> ShardedSSTableReader<T> {
    pub fn from_readers(
        readers: Vec<SSTableReader<T>>,
        min_key: &str,
        max_key: String,
    ) -> ShardedSSTableReader<T> {
        let mut reader = ShardedSSTableReader {
            readers: readers,
            max_key: max_key,
            map: BTreeMap::<String, (usize, Vec<u8>)>::new(),
            reached_end: false,
        };

        // First, seek to the starting key in all the SSTables.
        for r in reader.readers.iter_mut() {
            r.seek_to_min_key(min_key).unwrap();
        }

        // Next, we'll construct a BTreeMap using our keys. This will allow us to efficiently
        // insert while keeping a sorted list.
        for index in 0..reader.readers.len() {
            let mut data = match reader.readers[index].read_next_key().unwrap() {
                Some(x) => x,
                None => continue,
            };

            reader.map.insert(
                data.get_key().to_owned(),
                (index as usize, data.take_value()),
            );
        }

        reader
    }

    pub fn next(&mut self) -> Option<(String, T)> {
        if self.reached_end {
            return None;
        }

        let (key, value_bytes, index) = {
            // Now we'll pop off the lowest key and write it into the output sstable. Then we'll
            // refresh the key by reading from the sstable the key originated from.
            let (key, &(index, ref value)) = match self.map.iter().next() {
                Some(x) => x,
                None => {
                    self.reached_end = true;
                    return None;
                }
            };

            (key.clone(), value.clone(), index)
        };
        self.map.remove(key.as_str()).unwrap();

        match self.readers[index].read_next_key().unwrap() {
            Some(mut data) => {
                if self.max_key.as_str() == "" || data.get_key() < self.max_key.as_str() {
                    self.map.insert(
                        data.get_key().to_owned(),
                        (index as usize, data.take_value()),
                    );
                }
            }
            None => (),
        };

        if self.max_key.as_str() == "" || key < self.max_key {
            let mut value = T::default();
            value.read_from_bytes(&value_bytes).unwrap();
            Some((key, value))
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

impl<'a, T: Serializable + Default> Iterator for SpecdSSTableReader<'a, T> {
    type Item = (String, T);
    fn next(&mut self) -> Option<(String, T)> {
        if self.reached_end {
            return None;
        }

        while let Some(x) = self.reader.read_next_data().unwrap() {
            if x.0 < self.key_spec {
                continue;
            }
            let mode = self.is_within_scope(x.0.as_str());
            return match mode {
                0 => Some(x),
                1 => None,
                _ => continue,
            };
        }
        None
    }
}

impl<T: Serializable + Default> SSTableReader<T> {
    // read_next_key returns the next key and the unserialized bytes corresponding to the value.
    pub fn read_next_key(&mut self) -> io::Result<Option<sstable_proto_rust::DataEntry>> {
        if self.dtable_offset >= self.index_offset {
            return Ok(None);
        }

        let size = self.dtable.read_u32::<LittleEndian>()?;
        // Increment the dtable offset. We've read 4 bytes to read the size, plus we're about
        // to read size bytes, so we want to advance it by size + 4.
        self.dtable_offset += (size + 4) as u64;
        Ok(Some(
            protobuf::parse_from_reader(&mut (&mut self.dtable).take(size as u64)).map_err(
                |e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unable to parse KeyEntry protobuf in sstable: {}", e),
                    )
                },
            )?,
        ))
    }

    fn read_key(&mut self) -> io::Result<Option<String>> {
        let entry = match self.read_next_key()? {
            Some(e) => e,
            None => return Ok(None),
        };
        Ok(Some(entry.get_key().to_owned()))
    }

    fn read_next_data(&mut self) -> io::Result<Option<(String, T)>> {
        let key_entry = match self.read_next_key() {
            Ok(Some(k)) => k,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };

        let mut value = T::default();
        value.read_from_bytes(&key_entry.get_value()).unwrap();
        Ok(Some((key_entry.get_key().to_owned(), value)))
    }

    pub fn suggest_shards(&self, key_spec: &str, min_key: &str, max_key: &str) -> Vec<String> {
        index::suggest_shards(&self.index, key_spec, min_key, max_key)
    }
}

impl<T: Serializable + Default> Iterator for SSTableReader<T> {
    type Item = (String, T);
    fn next(&mut self) -> Option<(String, T)> {
        self.read_next_data().unwrap()
    }
}

mod index {
    pub fn get_block(
        index: &sstable_proto_rust::Index,
        key: &str,
    ) -> Option<sstable_proto_rust::KeyEntry> {
        match _get_block_index(index, key, false, false) {
            Some(i) => Some(index.pointers[i].to_owned()),
            None => None,
        }
    }

    // Suggest possible sharding points based on the contents of the index. The suggested sharding
    // points will be roughly of equal size. You should prefix and suffix with the  min and max
    // keys, then the suggested shards should be composed of the intervals between the resulting
    // keys. When using a key_spec, an implicit final shard should be included from the last key to
    // the end of the keyspec.
    pub fn suggest_shards(
        index: &sstable_proto_rust::Index,
        key_spec: &str,
        min_key: &str,
        max_key: &str,
    ) -> Vec<String> {
        let maybe_index: Option<usize>;
        let lower_bound = if key_spec > min_key {
            // In this case, we will use the key_spec to retrieve the block.
            maybe_index = get_block_index_with_keyspec(index, key_spec);
            key_spec
        } else {
            maybe_index = get_block_index_with_min_key(index, min_key);
            min_key
        };

        let mut boundaries = Vec::new();
        let mut sample_rate = 1;
        let mut count = 0;

        // Find the SSTable boundaries within the spec.
        if let Some(idx) = maybe_index {
            for i in (idx as usize)..index.get_pointers().len() {
                let ref keyentry = index.pointers[i];

                // Make sure we have passed the min key.
                if keyentry.get_key() < lower_bound {
                    continue;
                }

                // If we have passed the key spec, quit.
                if key_spec != "" && !keyentry.get_key().starts_with(key_spec) {
                    break;
                }

                // If we have passed the max key, quit.
                if max_key != "" && keyentry.get_key() > max_key {
                    break;
                }

                // If we start collecting loads of keys, downsample the amount we extract.
                // Arbitrarily start downsampling after extracting 1<<6 samples, which is 64.
                if boundaries.len() > (sample_rate << 6) {
                    sample_rate *= 2;
                }

                if count % sample_rate == 0 {
                    boundaries.push(keyentry.get_key().to_owned());
                }

                count += 1;
            }
        }

        boundaries
    }

    pub fn get_block_with_keyspec(
        index: &sstable_proto_rust::Index,
        key_spec: &str,
    ) -> Option<sstable_proto_rust::KeyEntry> {
        match _get_block_index(index, key_spec, true, false) {
            Some(i) => Some(index.pointers[i].to_owned()),
            None => None,
        }
    }

    // If we have a minimum key, jump to the first record either equal to or greater than the key.
    pub fn get_block_with_min_key(
        index: &sstable_proto_rust::Index,
        min_key: &str,
    ) -> Option<sstable_proto_rust::KeyEntry> {
        match _get_block_index(index, min_key, false, true) {
            Some(i) => Some(index.pointers[i].to_owned()),
            None => None,
        }
    }

    pub fn get_block_index_with_keyspec(
        index: &sstable_proto_rust::Index,
        key_spec: &str,
    ) -> Option<usize> {
        _get_block_index(index, key_spec, true, false)
    }

    pub fn get_block_index_with_min_key(
        index: &sstable_proto_rust::Index,
        min_key: &str,
    ) -> Option<usize> {
        _get_block_index(index, min_key, false, true)
    }

    // _get_block searches the index for a possible key. If a suitable block is found, it'll
    // return the byte offset for that block.
    fn _get_block_index(
        index: &sstable_proto_rust::Index,
        key: &str,
        as_key_spec: bool,
        as_min_key: bool,
    ) -> Option<usize> {
        let pointers = index.get_pointers();
        let length = pointers.len();
        if length == 0 {
            return None;
        }

        // First, we must find out the number of bits in the number.
        let mut bit_index = 1;
        while (length >> bit_index) > 0 {
            bit_index += 1
        }
        let mut i = 0;
        while bit_index > 0 {
            bit_index -= 1;
            i += 1 << bit_index;

            if i >= length || pointers[i].get_key() > key {
                // Unset the bit in question: we've gone too far down the list.
                i -= 1 << bit_index;
            } else if pointers[i].get_key() < key {
                // Do nothing, since we haven't gone far enough.
            } else {
                return Some(i);
            }
        }

        // For the case of using a key_spec, the key_spec is expected to rank higher than any value
        // fulfilling the spec. Therefore we may observe that the block key is higher than the
        // key_spec, which is acceptable as long as the block key matches the key spec.
        let allowable_for_key_spec = as_key_spec && (pointers[i].get_key().starts_with(key));

        match pointers[i].get_key() <= key || allowable_for_key_spec || as_min_key {
            true => Some(i),

            // If the block we found starts with a key which is already
            // higher than our key, that means our key doesn't exist.
            false => None,
        }
    }
}

pub fn mem_sstable(data: Vec<(String, u64)>) -> SSTableReader<Primitive<u64>> {
    let mut d = std::io::Cursor::new(Vec::new());
    {
        let mut t = SSTableBuilder::<Primitive<u64>>::new(&mut d);
        for (key, value) in data {
            t.write_ordered(key.as_str(), Primitive(value)).unwrap();
        }
        t.finish().unwrap();
    }
    d.seek(std::io::SeekFrom::Start(0)).unwrap();
    SSTableReader::<Primitive<u64>>::new(Box::new(d)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitive::*;
    use std::io::Seek;

    #[test]
    fn construct_sstable_builder() {
        let mut k = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut k);
            t.write_ordered("test", 123.into()).unwrap();
            t.finish().unwrap();
        }
        let bytes = k.into_inner();
        assert_eq!(
            bytes[bytes.len() - 8] + bytes[bytes.len() - 16] + 16,
            bytes.len() as u8
        );
    }

    #[test]
    fn serialize_i64() {
        let value: Primitive<i64> = Primitive(5);
        let x: &Serializable = &value;
        let mut k = std::io::Cursor::new(Vec::new());
        {
            x.write(&mut k).unwrap();
        }

        let mut z: Primitive<i64> = Primitive(9);
        {
            let y: &mut Serializable = &mut z;
            y.read_from_bytes(&k.into_inner()).unwrap();
        }

        assert_eq!(z, 5);
    }

    #[test]
    fn serialize_proto() {
        let mut value = sstable_proto_rust::KeyEntry::new();
        value.set_key(String::from("hello world"));
        value.set_offset(1234);

        let x: &Serializable = &value;
        let mut k = std::io::Cursor::new(Vec::new());
        {
            x.write(&mut k).unwrap();
        }

        let mut output = sstable_proto_rust::KeyEntry::new();
        {
            let y: &mut Serializable = &mut output;
            y.read_from_bytes(&k.into_inner()).unwrap();
        }

        assert_eq!(output.get_key(), "hello world");
        assert_eq!(output.get_offset(), 1234);
    }

    #[test]
    #[should_panic]
    fn construct_sstable_builder_backwards() {
        let mut k = std::io::Cursor::new(Vec::new());
        let mut t = SSTableBuilder::<Primitive<f64>>::new(&mut k);
        t.write_ordered("camel", Primitive(1.0234)).unwrap();
        // This write is out of order, which should be caught by an assertion.
        t.write_ordered("baboon", Primitive(0.222)).unwrap();
    }

    #[test]
    fn read_next_key_on_constructed_sstable() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d);
            t.write_ordered("hello", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let mut r = SSTableReader::<Primitive<i64>>::new(Box::new(d)).unwrap();
            let entry = r.read_next_key().unwrap().unwrap();
            assert_eq!(entry.get_key(), "hello");
        }
    }

    #[test]
    fn read_constructed_sstable_with_iter() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d);
            t.write_ordered("hello", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let r = SSTableReader::<Primitive<i64>>::new(Box::new(d)).unwrap();
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
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut f);
            for x in 0..100 {
                t.write_ordered(format!("{:9}", x).as_str(), Primitive(x))
                    .unwrap();
            }
            t.finish().unwrap();
        }
        let reader = SSTableReader::<Primitive<i64>>::new(Box::new(f)).unwrap();
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
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut f);
            t.write_ordered("a special key", Primitive(500)).unwrap();
            t.write_ordered("special key", Primitive(1234)).unwrap();
            t.write_ordered("zzz key", Primitive(400)).unwrap();
            t.finish().unwrap();
        }
        let mut reader = SSTableReader::<Primitive<i64>>::new(Box::new(f)).unwrap();
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
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut f);
            // Write 10k even numbers.
            for i in 0..10000 {
                t.write_ordered(format!("{}-->{:9}", key, i * 2).as_str(), Primitive(i * 2))
                    .unwrap();
            }
            t.finish().unwrap();
        }
        let mut reader = SSTableReader::<Primitive<i64>>::new(Box::new(f)).unwrap();
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
    fn merge_two_sstables() {
        let mut a = std::io::Cursor::new(Vec::new());
        let mut b = std::io::Cursor::new(Vec::new());
        let mut c = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut a);
            t.write_ordered("hello", Primitive(1)).unwrap();
            t.write_ordered("my name is Elder Price", Primitive(3))
                .unwrap();
            t.finish().unwrap();
        }
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut b);
            t.write_ordered("and I'd like to tell you...", Primitive(0))
                .unwrap();
            t.write_ordered("hello!", Primitive(2)).unwrap();
            t.finish().unwrap();
        }
        a.seek(std::io::SeekFrom::Start(0)).unwrap();
        b.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let mut r1 = SSTableReader::<Primitive<i64>>::new(Box::new(a)).unwrap();
            let mut r2 = SSTableReader::<Primitive<i64>>::new(Box::new(b)).unwrap();
            let mut merged = SSTableBuilder::from_sstables(&mut c, &mut [r1, r2]).unwrap();
        }
        c.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let mut merged = SSTableReader::<Primitive<i64>>::new(Box::new(c)).unwrap();
            assert_eq!(
                merged.read_next_key().unwrap().unwrap().get_key(),
                "and I'd like to tell you..."
            );
            assert_eq!(merged.read_next_key().unwrap().unwrap().get_key(), "hello");
            assert_eq!(merged.read_next_key().unwrap().unwrap().get_key(), "hello!");
            assert_eq!(
                merged.read_next_key().unwrap().unwrap().get_key(),
                "my name is Elder Price"
            );
        }
    }

    #[test]
    fn read_with_nonexistent_key_spec() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d);
            t.write_ordered("allow", Primitive(5)).unwrap();
            t.write_ordered("bellow", Primitive(5)).unwrap();
            t.write_ordered("wallow", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let mut r = SSTableReader::<Primitive<i64>>::new(Box::new(d)).unwrap();
            let specd_reader = SpecdSSTableReader::from_reader(&mut r, "hello-");
            assert_eq!(
                specd_reader.map(|(k, v)| k).collect::<Vec<String>>(),
                Vec::<String>::new()
            );
        }
    }

    #[test]
    fn read_with_key_spec() {
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d);
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
            let mut r = SSTableReader::<Primitive<i64>>::new(Box::new(d)).unwrap();
            let specd_reader = SpecdSSTableReader::from_reader(&mut r, "hello-");
            assert_eq!(
                specd_reader.map(|(k, v)| k).collect::<Vec<_>>(),
                vec!["hello-1", "hello-2", "hello-3"]
            );
        }
    }

    #[test]
    fn test_within_scope() {
        // Construct an empty SSTable.
        let mut d = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d);
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();

        let mut r = SSTableReader::<Primitive<i64>>::new(Box::new(d)).unwrap();
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
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d);
            t.write_ordered("allow", Primitive(5)).unwrap();
            t.write_ordered("bellow", Primitive(5)).unwrap();
            t.write_ordered("hello-1", Primitive(5)).unwrap();
            t.write_ordered("hello-2", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d.seek(std::io::SeekFrom::Start(0)).unwrap();
        {
            let mut r = SSTableReader::<Primitive<i64>>::new(Box::new(d)).unwrap();
            r.seek_to_min_key("c");
            assert_eq!(
                r.map(|(k, v)| k).collect::<Vec<_>>(),
                vec!["hello-1", "hello-2"]
            );
        }
    }

    #[test]
    fn test_sharded_read() {
        let mut d1 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d1);
            t.write_ordered("aardvark", Primitive(5)).unwrap();
            t.write_ordered("bee", Primitive(5)).unwrap();
            t.write_ordered("cat", Primitive(5)).unwrap();
            t.write_ordered("dog", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d1.seek(std::io::SeekFrom::Start(0)).unwrap();

        let mut d2 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d2);
            t.write_ordered("apple", Primitive(5)).unwrap();
            t.write_ordered("banana", Primitive(5)).unwrap();
            t.write_ordered("cantaloupe", Primitive(5)).unwrap();
            t.write_ordered("durian", Primitive(5)).unwrap();
            t.finish().unwrap();
        }
        d2.seek(std::io::SeekFrom::Start(0)).unwrap();

        {
            let mut r1 = SSTableReader::<Primitive<i64>>::new(Box::new(d1)).unwrap();
            let mut r2 = SSTableReader::<Primitive<i64>>::new(Box::new(d2)).unwrap();

            let mut s = ShardedSSTableReader::<Primitive<i64>>::from_readers(
                vec![r1, r2],
                "c",
                String::from(""),
            );

            assert_eq!(
                s.map(|(k, v)| k).collect::<Vec<_>>(),
                vec!["cantaloupe", "cat", "dog", "durian"]
            );
        }
    }

    #[test]
    fn test_sharded_read_with_max_key() {
        let mut d1 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d1);
            t.write_ordered("aardvark", Primitive(5)).unwrap();
            t.write_ordered("bee", Primitive(5)).unwrap();
            t.write_ordered("cat", Primitive(5)).unwrap();
            t.write_ordered("dog", Primitive(5)).unwrap();
            t.finish().unwrap();
        }

        let mut d2 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Primitive<i64>>::new(&mut d2);
            t.write_ordered("apple", Primitive(5)).unwrap();
            t.write_ordered("banana", Primitive(5)).unwrap();
            t.write_ordered("cantaloupe", Primitive(5)).unwrap();
            t.write_ordered("durian", Primitive(5)).unwrap();
            t.finish().unwrap();
        }

        {
            let mut r1 = SSTableReader::<Primitive<i64>>::new(Box::new(d1)).unwrap();
            let mut r2 = SSTableReader::<Primitive<i64>>::new(Box::new(d2)).unwrap();
            let mut s = ShardedSSTableReader::<Primitive<i64>>::from_readers(
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
}

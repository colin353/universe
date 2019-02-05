extern crate byteorder;
extern crate protobuf;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Read;

pub struct RecordIOWriter<T: protobuf::Message, W: std::io::Write + Send + Sync> {
    writer: W,

    // We have to explicitly state that the struct uses the type T, or else the rust compiler will
    // get confused. This is a zero-size type to help the compiler infer the usage of T.
    data_type: std::marker::PhantomData<T>,
}

impl<T: protobuf::Message, W: std::io::Write + Send + Sync> RecordIOWriter<T, W> {
    pub fn new(writer: W) -> Self {
        RecordIOWriter {
            writer: writer,
            data_type: std::marker::PhantomData,
        }
    }

    pub fn write(&mut self, record: &T) {
        // First four bytes is the amount of bytes to read.
        let size = record.compute_size();
        self.writer
            .write_u32::<LittleEndian>(size)
            .expect("failed to write protobuf size?");

        // Then we have the protobuf bytes themselves.
        record.write_to_writer(&mut self.writer).unwrap();
    }
}

pub struct RecordIOReader<T: protobuf::Message, R: std::io::Read> {
    reader: R,

    // We have to explicitly state that the struct uses the type T, or else the rust compiler will
    // get confused. This is a zero-size type to help the compiler infer the usage of T.
    data_type: std::marker::PhantomData<T>,
}

impl<T: protobuf::Message, R: std::io::Read> RecordIOReader<T, R> {
    pub fn new(reader: R) -> Self {
        RecordIOReader {
            reader: reader,
            data_type: std::marker::PhantomData,
        }
    }

    pub fn read(&mut self) -> Option<T> {
        // First four bytes is the amount of bytes to read. If we reach
        // EOF, just return None.
        let size = match self.reader.read_u32::<LittleEndian>() {
            Ok(x) => x,
            Err(_) => return None,
        };

        Some(protobuf::parse_from_reader(&mut (&mut self.reader).take(size as u64)).unwrap())
    }
}

impl<T: protobuf::Message, R: std::io::Read> Iterator for RecordIOReader<T, R> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.read()
    }
}

pub type RecordIOReaderOwned<T> = RecordIOReader<T, Box<std::io::Read>>;
pub type RecordIOReaderBorrowed<'a, T> = RecordIOReader<T, &'a std::io::Read>;
pub type RecordIOWriterOwned<T> = RecordIOWriter<T, Box<std::io::Write + Send + Sync>>;
pub type RecordIOWriterBorrowed<'a, T> = RecordIOWriter<T, &'a (std::io::Write + Send + Sync)>;

#[cfg(test)]
mod tests {
    use super::*;

    use protobuf::well_known_types::Any;
    use std::io;
    use std::io::Seek;

    fn make_test_msg(content: &str) -> Any {
        let mut msg = Any::new();
        msg.set_type_url(content.to_owned());
        msg
    }

    #[test]
    fn write_recordio_owned() {
        let mut k = std::io::Cursor::new(Vec::new());
        let k = {
            let mut w = RecordIOWriter::new(Box::new(k));
            w.write(&make_test_msg("hello world"));
            w.write(&make_test_msg("second message"));
        };
    }

    #[test]
    fn construct_recordio_builder() {
        let mut k = std::io::Cursor::new(Vec::new());
        {
            let mut w = RecordIOWriter::new(&mut k as &mut (std::io::Write + Send + Sync));
            w.write(&make_test_msg("hello world"));
            w.write(&make_test_msg("second message"));
        }

        // Reset the cursor to the start.
        k.seek(io::SeekFrom::Start(0)).unwrap();
        let mut r = RecordIOReaderOwned::<Any>::new(Box::new(k));
        assert_eq!(r.read().unwrap().get_type_url(), "hello world");
        assert_eq!(r.read().unwrap().get_type_url(), "second message");
        assert!(r.read().is_none());
    }

    #[test]
    fn test_iteration() {
        let mut k = std::io::Cursor::new(Vec::new());
        {
            let mut w = RecordIOWriter::new(Box::new(&mut k));
            w.write(&make_test_msg("1"));
            w.write(&make_test_msg("2"));
            w.write(&make_test_msg("3"));
        }
        // Reset the cursor to the start.
        k.seek(io::SeekFrom::Start(0)).unwrap();
        let mut r = RecordIOReader::<Any, _>::new(Box::new(k));

        // Read using an iterator.
        assert_eq!(
            r.map(|x| x.get_type_url().to_owned()).collect::<Vec<_>>(),
            vec!["1", "2", "3"],
        );
    }
}

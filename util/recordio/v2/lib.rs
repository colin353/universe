use bus::{DeserializeOwned, Serialize};

pub struct RecordIOBuilder<T, W: std::io::Write> {
    writer: W,
    _marker: std::marker::PhantomData<T>,
}

pub struct RecordIOReader<T, R: std::io::Read> {
    reader: R,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Serialize, W: std::io::Write> RecordIOBuilder<T, W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn write(&mut self, record: &T) -> std::io::Result<()> {
        let mut buf = Vec::new();
        record.encode(&mut buf)?;

        let size = buf.len() as u32;
        self.writer.write_all(&size.to_le_bytes())?;
        self.writer.write_all(&buf)
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl<'a, T: DeserializeOwned, R: std::io::Read> RecordIOReader<T, R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn next(&mut self) -> Option<std::io::Result<T>> {
        let mut size = [0; 4];
        self.reader.read_exact(&mut size).ok()?;
        let size = u32::from_le_bytes(size);

        let mut buf = vec![0; size as usize];
        self.reader.read_exact(&mut buf).ok()?;

        Some(T::decode_owned(&buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_recordio() {
        let mut buf = Vec::new();
        {
            let mut w = RecordIOBuilder::new(&mut buf);
            w.write("asdf").unwrap();
            w.write("fdsa").unwrap();
        }

        let c = std::io::Cursor::new(&buf);
        let mut r = RecordIOReader::<String, _>::new(c);
        assert_eq!(&r.next().unwrap().unwrap(), "asdf");
        assert_eq!(&r.next().unwrap().unwrap(), "fdsa");
        assert!(r.next().is_none());
    }

    #[test]
    fn write_recordio_owned() {
        let buf = Vec::new();
        let mut w = RecordIOBuilder::new(buf);
        w.write("asdf").unwrap();
        w.write("fdsa").unwrap();
    }
}

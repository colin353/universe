use crate::varint;

pub trait Serialize: Sized {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error>;
}

pub trait Deserialize<'a>: Sized {
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error>;
}

pub trait DeserializeOwned: Sized {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error>;
}

impl<'a, T: DeserializeOwned> Deserialize<'a> for T {
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        T::decode_owned(bytes)
    }
}

impl<T: Serialize> Serialize for &T {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        self.encode(writer)
    }
}

impl Serialize for u8 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        if *self == 0 {
            return Ok(0);
        }
        writer.write_all(&[*self])?;
        Ok(1)
    }
}

impl DeserializeOwned for u8 {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        if bytes.len() == 0 {
            return Ok(0);
        }
        Ok(bytes[0])
    }
}

impl Serialize for u16 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_varint(*self as u64, writer)
    }
}

impl DeserializeOwned for u16 {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_varint(bytes);
        Ok(x as u16)
    }
}

impl Serialize for u32 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_varint(*self as u64, writer)
    }
}

impl DeserializeOwned for u32 {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_varint(bytes);
        Ok(x as u32)
    }
}

impl Serialize for u64 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_varint(*self, writer)
    }
}

impl DeserializeOwned for u64 {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_varint(bytes);
        Ok(x as u64)
    }
}

impl<'a> Serialize for String {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let buf = self.as_bytes();
        writer.write_all(buf)?;
        Ok(buf.len())
    }
}

impl DeserializeOwned for String {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        String::from_utf8(bytes.to_owned())
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))
    }
}

impl<'a> Serialize for &'a str {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let buf = self.as_bytes();
        writer.write_all(buf)?;
        Ok(buf.len())
    }
}

impl<'a> Deserialize<'a> for &'a str {
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        std::str::from_utf8(bytes)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_encode_decode<'a, T: Serialize + Deserialize<'a> + std::fmt::Debug + PartialEq>(
        buf: &'a mut Vec<u8>,
        value: T,
    ) {
        buf.clear();
        value.encode(buf).unwrap();
        let decoded = T::decode(buf).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn test_encoding() {
        let mut buf = Vec::new();
        test_encode_decode(&mut buf, "asdf");
        test_encode_decode(&mut buf, 123 as u8);
        test_encode_decode(&mut buf, 123 as u16);
        test_encode_decode(&mut buf, 123 as u32);
        test_encode_decode(&mut buf, 123 as u64);

        test_encode_decode(&mut buf, 456 as u16);
        test_encode_decode(&mut buf, 456 as u32);
        test_encode_decode(&mut buf, 456 as u64);

        test_encode_decode(&mut buf, 456789 as u32);
        test_encode_decode(&mut buf, 456789 as u64);

        test_encode_decode(&mut buf, u64::MAX);
    }
}

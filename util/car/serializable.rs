use crate::varint;

pub trait Serializable<'a>: Sized {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error>;
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error>;
    fn zero() -> Self;
}

impl<'a> Serializable<'a> for u8 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        if *self == 0 {
            return Ok(0);
        }
        writer.write_all(&[*self])?;
        Ok(1)
    }

    fn decode(bytes: &[u8]) -> Result<Self, std::io::Error> {
        if bytes.len() == 0 {
            return Ok(0);
        }
        Ok(bytes[0])
    }

    fn zero() -> u8 {
        0
    }
}

impl<'a> Serializable<'a> for u16 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_varint(*self as u64, writer)
    }

    fn decode(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_varint(bytes);
        Ok(x as u16)
    }

    fn zero() -> u16 {
        0
    }
}

impl<'a> Serializable<'a> for u32 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_varint(*self as u64, writer)
    }

    fn decode(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_varint(bytes);
        Ok(x as u32)
    }

    fn zero() -> u32 {
        0
    }
}

impl<'a> Serializable<'a> for u64 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_varint(*self, writer)
    }

    fn decode(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_varint(bytes);
        Ok(x as u64)
    }

    fn zero() -> u64 {
        0
    }
}

impl<'a> Serializable<'a> for &'a str {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let buf = self.as_bytes();
        writer.write_all(buf)?;
        Ok(buf.len())
    }

    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        std::str::from_utf8(bytes)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))
    }

    fn zero() -> &'a str {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_encode_decode<'a, T: Serializable<'a> + std::fmt::Debug + PartialEq>(
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

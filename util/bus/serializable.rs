use crate::varint;

#[derive(Debug, Clone)]
pub struct Nothing {}

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
        (*self).encode(writer)
    }
}

impl Serialize for Nothing {
    fn encode<W: std::io::Write>(&self, _w: &mut W) -> Result<usize, std::io::Error> {
        return Ok(0);
    }
}

impl DeserializeOwned for Nothing {
    fn decode_owned(_b: &[u8]) -> Result<Self, std::io::Error> {
        Ok(Nothing {})
    }
}

impl Serialize for i64 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_ivarint(*self, writer)
    }
}

impl DeserializeOwned for i64 {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_ivarint(bytes);
        Ok(x)
    }
}

impl Serialize for i32 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_ivarint(*self as i64, writer)
    }
}

impl DeserializeOwned for i32 {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_ivarint(bytes);
        Ok(x as i32)
    }
}

impl Serialize for i16 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        varint::encode_ivarint(*self as i64, writer)
    }
}

impl DeserializeOwned for i16 {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let (x, _) = varint::decode_ivarint(bytes);
        Ok(x as i16)
    }
}

impl Serialize for i8 {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        if *self == 0 {
            return Ok(0);
        }
        let byte_representation = unsafe { std::mem::transmute::<i8, u8>(*self) };
        writer.write_all(&[byte_representation])?;
        Ok(1)
    }
}

impl DeserializeOwned for i8 {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        if bytes.len() == 0 {
            return Ok(0);
        }
        Ok(unsafe { std::mem::transmute::<u8, i8>(bytes[0]) })
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

impl Serialize for bool {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        if *self {
            writer.write_all(&[1])?;
            return Ok(1);
        }
        Ok(0)
    }
}

impl DeserializeOwned for bool {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        Ok(!bytes.is_empty())
    }
}

// Packed... is a wrapper struct to distinguish between Vec<T: Serializable> and fixed size Vecs,
// e.g. Vec<u8> or Vec<u16>, which should just be written packed together, without an EncodedStruct
// wrapping them.
pub struct PackedIn<T>(pub Vec<T>);
pub struct PackedOut<'a, T>(pub &'a [T]);

impl<T> Default for PackedIn<T> {
    fn default() -> Self {
        PackedIn(Vec::new())
    }
}

impl<'a> Serialize for PackedOut<'a, u8> {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        writer.write_all(&self.0)?;
        Ok(self.0.len())
    }
}

impl Serialize for PackedIn<u8> {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        writer.write_all(&self.0)?;
        Ok(self.0.len())
    }
}

impl DeserializeOwned for PackedIn<u8> {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        Ok(PackedIn(bytes.to_owned()))
    }
}

impl<'a> Serialize for &'a [u8] {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        println!("encode {:?} ({} bytes)", self, self.len());
        writer.write_all(self)?;
        Ok(self.len())
    }
}

impl<'a> Deserialize<'a> for &'a [u8] {
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(bytes)
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

        test_encode_decode(&mut buf, "asdf");
        test_encode_decode(&mut buf, -123 as i8);
        test_encode_decode(&mut buf, -123 as i16);
        test_encode_decode(&mut buf, -123 as i32);
        test_encode_decode(&mut buf, -123 as i64);

        test_encode_decode(&mut buf, -456 as i16);
        test_encode_decode(&mut buf, -456 as i32);
        test_encode_decode(&mut buf, -456 as i64);

        test_encode_decode(&mut buf, -456789 as i32);
        test_encode_decode(&mut buf, -456789 as i64);

        test_encode_decode(&mut buf, -i64::MAX);
    }
}

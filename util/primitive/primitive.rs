/*
 * primitive.rs
 *
 * This code implements serialization for primitive types.
 */

pub extern crate primitive_proto_rust;
extern crate protobuf;
use protobuf::Message;

use std::io;

// A serializable object can be converted back and forth from a byte stream.
pub trait Serializable {
    fn write(&self, write: &mut std::io::Write) -> io::Result<u64>;
    fn read_from_bytes(&mut self, buffer: &[u8]) -> io::Result<()>;
}

// Wrapper for primitive types, so they can be serialized using a standard
// method.
#[derive(Clone, Copy, Eq, Debug)]
pub struct Primitive<N>(pub N);

pub trait PrimitiveType {}
impl PrimitiveType for u64 {}
impl PrimitiveType for i64 {}
impl PrimitiveType for f64 {}
impl PrimitiveType for String {}
impl PrimitiveType for Vec<u8> {}

impl<T: PrimitiveType> From<T> for Primitive<T> {
    fn from(x: T) -> Self {
        Primitive(x)
    }
}

impl<T: Default> Default for Primitive<T> {
    fn default() -> Self {
        Primitive(T::default())
    }
}

impl<T: PartialEq> PartialEq<T> for Primitive<T> {
    fn eq(&self, other: &T) -> bool {
        self.0.eq(other)
    }

    fn ne(&self, other: &T) -> bool {
        self.0.ne(other)
    }
}

impl<T: PartialEq> PartialEq<Primitive<T>> for Primitive<T> {
    fn eq(&self, other: &Primitive<T>) -> bool {
        self.0.eq(&**other)
    }

    fn ne(&self, other: &Primitive<T>) -> bool {
        self.0.ne(&**other)
    }
}

// So Primitive<N> can be used like an N.
impl<N> std::ops::Deref for Primitive<N> {
    type Target = N;
    fn deref(&self) -> &N {
        &self.0
    }
}

// So Primitive<N> can be used like an N.
impl<N> std::ops::DerefMut for Primitive<N> {
    fn deref_mut(&mut self) -> &mut N {
        &mut self.0
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Primitive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serializable for Primitive<u64> {
    fn read_from_bytes(&mut self, buffer: &[u8]) -> std::io::Result<()> {
        match protobuf::parse_from_bytes::<primitive_proto_rust::UnsignedNumber>(buffer) {
            Ok(number) => {
                **self = number.get_number();
                Ok(())
            }
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unable to deserialize u64",
            )),
        }
    }

    fn write(&self, w: &mut std::io::Write) -> io::Result<u64> {
        let mut number = primitive_proto_rust::UnsignedNumber::new();
        number.set_number(*self.to_owned());
        match number.write_to_writer(w) {
            Ok(_) => Ok(number.get_cached_size() as u64),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unable to serialize number!",
            )),
        }
    }
}
impl Serializable for Primitive<i64> {
    fn read_from_bytes(&mut self, buffer: &[u8]) -> std::io::Result<()> {
        match protobuf::parse_from_bytes::<primitive_proto_rust::Number>(buffer) {
            Ok(number) => {
                **self = number.get_number();
                Ok(())
            }
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unable to deserialize i64",
            )),
        }
    }

    fn write(&self, w: &mut std::io::Write) -> io::Result<u64> {
        let mut number = primitive_proto_rust::Number::new();
        number.set_number(*self.to_owned());
        match number.write_to_writer(w) {
            Ok(_) => Ok(number.get_cached_size() as u64),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unable to serialize number!",
            )),
        }
    }
}

impl Serializable for Primitive<f64> {
    fn write(&self, w: &mut std::io::Write) -> io::Result<u64> {
        let mut float = primitive_proto_rust::Float::new();
        float.set_number(*self.to_owned());
        match float.write_to_writer(w) {
            Ok(_) => Ok(float.get_cached_size() as u64),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unable to serialize number!",
            )),
        }
    }

    fn read_from_bytes(&mut self, buffer: &[u8]) -> io::Result<()> {
        match protobuf::parse_from_bytes::<primitive_proto_rust::Float>(buffer) {
            Ok(number) => {
                **self = number.get_number();
                Ok(())
            }
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unable to deserialize f64",
            )),
        }
    }
}

impl Serializable for Primitive<Vec<u8>> {
    fn write(&self, w: &mut std::io::Write) -> io::Result<u64> {
        w.write_all(self)?;
        Ok(self.len() as u64)
    }

    fn read_from_bytes(&mut self, buffer: &[u8]) -> io::Result<()> {
        **self = buffer.to_owned();
        Ok(())
    }
}

impl Serializable for Primitive<String> {
    fn write(&self, w: &mut std::io::Write) -> io::Result<u64> {
        w.write_all(&self.0.as_bytes())?;
        Ok(self.0.len() as u64)
    }

    fn read_from_bytes(&mut self, buffer: &[u8]) -> io::Result<()> {
        self.0 = String::from_utf8(buffer.to_owned()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Unable to parse string as utf-8.",
            )
        })?;
        Ok(())
    }
}

impl<T: protobuf::Message> Serializable for T {
    fn write(&self, w: &mut std::io::Write) -> io::Result<u64> {
        match self.write_to_writer(w) {
            Ok(_) => Ok(self.get_cached_size() as u64),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unable to serialize protobuf",
            )),
        }
    }

    fn read_from_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.merge_from_bytes(bytes).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unable to deserialize protobuf",
            )
        })
    }
}

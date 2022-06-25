use crate::pack;
use crate::varint;
use crate::{Deserialize, DeserializeOwned, Serialize};

pub enum RepeatedField<'a, T> {
    Encoded(EncodedStruct<'a>),
    DecodedOwned(Vec<T>),
    DecodedReference(&'a [T]),
}

#[derive(Clone)]
pub struct EncodedStruct<'a> {
    data: &'a [u8],
    fields_index: pack::Pack<'a>,
    empty: bool,
}

pub struct EncodedStructBuilder<W: std::io::Write> {
    sizes: Vec<u32>,
    writer: W,
}

impl<W: std::io::Write> EncodedStructBuilder<W> {
    pub fn new(writer: W) -> Self {
        Self {
            sizes: Vec::new(),
            writer,
        }
    }

    pub fn push<'a, T: Serialize>(&mut self, value: &T) -> Result<(), std::io::Error> {
        let length = value.encode(&mut self.writer)?;
        self.sizes.push(length as u32);
        Ok(())
    }

    pub fn finish(mut self) -> Result<usize, std::io::Error> {
        if self.sizes.is_empty() {
            return Ok(0);
        }

        let mut pack = pack::PackBuilder::new(&mut self.writer);
        for size in &self.sizes[0..self.sizes.len()] {
            pack.push(*size)?;
        }

        let pack_size = pack.finish()?;

        // Write the footer, which is the number of encoded elements
        let footer_size = varint::encode_reverse_varint((pack_size + 1) as u32, &mut self.writer)?;

        let data_size: u32 = self.sizes.iter().sum();
        Ok(data_size as usize + pack_size + footer_size)
    }
}

impl<'a> Serialize for EncodedStruct<'a> {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        writer.write_all(&self.data)?;
        let pack_size = self.fields_index.encode(writer)?;
        let footer_size = varint::encode_reverse_varint((pack_size + 1) as u32, writer)?;
        Ok(self.data.len() + pack_size + footer_size)
    }
}

impl<'a> Deserialize<'a> for EncodedStruct<'a> {
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::new(bytes)?)
    }
}

impl<'a> EncodedStruct<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, std::io::Error> {
        let (footer, footer_size) = varint::decode_reverse_varint(data);
        if footer == 0 {
            return Ok(Self {
                empty: true,
                data: &[],
                fields_index: pack::Pack::new(&[])?,
            });
        }

        let pack_size = footer - 1;
        println!(
            "data.len(): {}, pack_size: {}, footer_size: {}",
            data.len(),
            pack_size,
            footer_size
        );
        let data_length = data.len() - pack_size - footer_size;
        Ok(Self {
            empty: false,
            data: &data[0..data_length],
            fields_index: pack::Pack::new(&data[data_length..data.len() - footer_size])?,
        })
    }

    pub fn get_struct(&'a self, idx: usize) -> Option<Result<Self, std::io::Error>> {
        if self.empty {
            return None;
        }

        let start = if idx == 0 {
            0
        } else {
            self.fields_index.get(idx - 1)? as usize
        };

        let end = if let Some(end) = self.fields_index.get(idx) {
            end as usize
        } else {
            self.data.len()
        };

        Some(EncodedStruct::new(&self.data[start..end]))
    }

    pub fn get_owned<T: DeserializeOwned>(&self, idx: usize) -> Option<Result<T, std::io::Error>> {
        if self.empty {
            return None;
        }

        let start = if idx == 0 {
            0
        } else {
            self.fields_index.get(idx - 1)? as usize
        };

        let end = if let Some(end) = self.fields_index.get(idx) {
            end as usize
        } else {
            self.data.len()
        };

        Some(T::decode_owned(&self.data[start..end]))
    }

    pub fn get<T: Deserialize<'a>>(&'a self, idx: usize) -> Option<Result<T, std::io::Error>> {
        if self.empty {
            return None;
        }

        let start = if idx == 0 {
            0
        } else {
            self.fields_index.get(idx - 1)? as usize
        };

        let end = if let Some(end) = self.fields_index.get(idx) {
            end as usize
        } else {
            self.data.len()
        };

        println!("get: {:?}", &self.data[start..end]);

        Some(T::decode(&self.data[start..end]))
    }

    pub fn len(&self) -> usize {
        if self.empty {
            0
        } else {
            self.fields_index.len() + 1
        }
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut builder = EncodedStructBuilder::new(writer);
        for element in self {
            builder.push(element)?;
        }
        builder.finish()
    }
}

impl<T: DeserializeOwned> DeserializeOwned for Vec<T> {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let e = EncodedStruct::new(bytes)?;
        let mut out = Vec::new();
        // TODO: use iterators instead!
        for i in 0..e.len() {
            out.push(e.get_owned(i).unwrap()?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repeated_field() {
        let mut buf = Vec::new();
        let mut b = EncodedStructBuilder::new(&mut buf);
        b.push("asdf").unwrap();
        b.push("fdsa").unwrap();
        b.finish().unwrap();

        let rf = EncodedStruct::new(&buf).unwrap();
        assert_eq!(rf.len(), 2);
        assert_eq!(rf.is_empty(), false);
        assert_eq!(rf.get::<&str>(0).unwrap().unwrap(), "asdf");
        assert_eq!(rf.get::<&str>(1).unwrap().unwrap(), "fdsa");
        assert_eq!(rf.get::<&str>(2).is_none(), true);
    }

    #[test]
    fn test_empty_repeated_field() {
        let mut buf = Vec::new();
        let mut b = EncodedStructBuilder::new(&mut buf);
        b.finish().unwrap();

        let rf = EncodedStruct::new(&buf).unwrap();
        assert_eq!(rf.len(), 0);
        assert_eq!(rf.is_empty(), true);
        assert_eq!(rf.get::<&str>(0).is_none(), true);
    }

    #[test]
    fn test_repeated_field_one_empty_item() {
        let mut buf = Vec::new();
        let mut b = EncodedStructBuilder::new(&mut buf);
        b.push("").unwrap(); // encoded size: zero
        b.finish().unwrap();

        let rf = EncodedStruct::new(&buf).unwrap();
        assert_eq!(rf.len(), 1);
        assert_eq!(rf.is_empty(), false);
        assert_eq!(rf.get::<&str>(0).unwrap().unwrap(), "");
    }
}

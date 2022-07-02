use crate::pack;
use crate::varint;
use crate::{Deserialize, DeserializeOwned, Serialize};

pub enum RepeatedField<'a, T> {
    Encoded(EncodedStruct<'a>),
    DecodedOwned(Vec<T>),
    DecodedReference(&'a [T]),
}

impl<'a, T: std::fmt::Debug + DeserializeOwned> std::fmt::Debug for RepeatedField<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(_) => {
                write!(f, "[")?;
                let mut iter = self.iter();
                let mut next = iter.next();
                while let Some(item) = next {
                    if let Ok(i) = item.as_ref() {
                        write!(f, "{:?}", i)?;
                    } else {
                        write!(f, "??")?;
                    }
                    next = iter.next();
                    if next.is_some() {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")
            }
            Self::DecodedOwned(v) => v.fmt(f),
            Self::DecodedReference(v) => v.fmt(f),
        }
    }
}

impl<'a, T> RepeatedField<'a, T>
where
    &'a T: Deserialize<'a>,
{
    pub fn get(&'a self, index: usize) -> Option<&'a T> {
        match self {
            RepeatedField::Encoded(s) => s.get(index).map(|x| x.unwrap()),
            RepeatedField::DecodedOwned(v) => Some(&v[index]),
            RepeatedField::DecodedReference(v) => Some(&v[index]),
        }
    }
}

impl<'a> RepeatedField<'a, u64> {
    pub fn get(&'a self, index: usize) -> Option<u64> {
        match self {
            RepeatedField::Encoded(s) => s.get(index).map(|x| x.unwrap()),
            RepeatedField::DecodedOwned(v) => Some(v[index]),
            RepeatedField::DecodedReference(v) => Some(v[index]),
        }
    }
}

impl<'a, T> RepeatedField<'a, T>
where
    T: DeserializeOwned,
{
    pub fn iter(&'a self) -> RepeatedFieldIterator<'a, T> {
        match self {
            Self::Encoded(e) => RepeatedFieldIterator::Encoded(e.iter()),
            Self::DecodedOwned(i) => RepeatedFieldIterator::DecodedOwned(i.iter()),
            Self::DecodedReference(i) => RepeatedFieldIterator::DecodedReference(i.iter()),
        }
    }
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

    pub fn push<'a, T: Serialize>(&mut self, value: T) -> Result<(), std::io::Error> {
        let length = value.encode(&mut self.writer)?;
        self.sizes.push(length as u32);
        Ok(())
    }

    pub fn finish(mut self) -> Result<usize, std::io::Error> {
        if self.sizes.is_empty() {
            return Ok(0);
        }

        let mut pack = pack::PackBuilder::new(&mut self.writer);
        for size in &self.sizes[0..self.sizes.len() - 1] {
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
    // The EncodedStruct layout is:
    //
    // [ data u8 ... ] [ Pack ... ] [ footer ]
    //
    // See the Pack data structure for info on that layout. The pack contains
    // offsets for each field in the data payload. The footer describes the
    // length of the pack.
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

        println!("get: {:?}", &self.data);

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

    pub fn iter(&'a self) -> EncodedStructIterator<'a> {
        EncodedStructIterator {
            data: &self.data,
            last_offset: 0,
            pack_iter: self.fields_index.iter(),
            done: false,
            data_size: self.data.len() as usize,
        }
    }
}

pub struct EncodedStructIterator<'a> {
    data: &'a [u8],
    last_offset: usize,
    pack_iter: pack::PackIterator<'a>,
    done: bool,
    data_size: usize,
}

impl<'a> EncodedStructIterator<'a> {
    pub fn get(&self, start: usize, end: usize) -> &'a [u8] {
        &self.data[start..end]
    }
}

impl<'a> Iterator for EncodedStructIterator<'a> {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<(usize, usize)> {
        if self.done {
            return None;
        }

        let start = self.last_offset;
        let end = match self.pack_iter.next() {
            Some(end) => end as usize,
            None => {
                self.done = true;
                self.data_size
            }
        };

        self.last_offset = end;
        Some((start, end))
    }
}

pub enum RefContainer<'a, T> {
    Reference(&'a T),
    Owned(Box<T>),
    DecodingFailed,
}

impl<'a, T> RefContainer<'a, T> {
    fn as_ref(&'a self) -> Result<&'a T, std::io::Error> {
        match self {
            Self::Reference(r) => Ok(r),
            Self::Owned(b) => Ok(b.as_ref()),
            Self::DecodingFailed => Err(std::io::Error::from(std::io::ErrorKind::InvalidData)),
        }
    }
}

pub enum RepeatedFieldIterator<'a, T> {
    Encoded(EncodedStructIterator<'a>),
    DecodedOwned(std::slice::Iter<'a, T>),
    DecodedReference(std::slice::Iter<'a, T>),
}

impl<'a, T: DeserializeOwned> Iterator for RepeatedFieldIterator<'a, T> {
    type Item = RefContainer<'a, T>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(si) => {
                let (start, end) = si.next()?;
                Some(RefContainer::Owned(Box::new(
                    T::decode_owned(si.get(start, end)).unwrap(),
                )))
            }
            Self::DecodedOwned(i) => Some(RefContainer::Reference(i.next()?)),
            Self::DecodedReference(i) => Some(RefContainer::Reference(i.next()?)),
        }
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
        for (start, end) in e.iter() {
            out.push(T::decode_owned(&e.data[start..end])?);
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

    #[test]
    fn test_field_index_iteration() {
        let mut buf = Vec::new();
        let mut b = EncodedStructBuilder::new(&mut buf);
        b.push("hello to the world").unwrap(); // encoded size: 18 bytes
        b.push("some more data").unwrap(); // encoded size: 14 bytes
        b.push("additional stuff").unwrap(); // encoded size: 16 bytes
        b.finish().unwrap();

        let rf = EncodedStruct::new(&buf).unwrap();
        let mut iter = rf.iter();
        assert_eq!(iter.next(), Some((0, 18)));
        assert_eq!(iter.next(), Some((18, 32)));
        assert_eq!(iter.next(), Some((32, 48)));
        assert_eq!(iter.next(), None);
    }
}

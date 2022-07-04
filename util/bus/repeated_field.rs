use crate::encoded_struct::{EncodedStruct, EncodedStructBuilder, EncodedStructIterator};
use crate::{Deserialize, DeserializeOwned, Serialize};

pub enum RepeatedField<'a, T> {
    Encoded(EncodedStruct<'a>),
    Decoded(&'a [T]),
}

impl<'a, T: std::fmt::Debug + Copy + DeserializeOwned> std::fmt::Debug for RepeatedField<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(_) => {
                write!(f, "[")?;
                let z = &self;
                let mut iter = z.into_iter();
                let mut next = iter.next();
                while let Some(item) = next {
                    write!(f, "{:?}", item)?;
                    next = iter.next();
                    if next.is_some() {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")
            }
            Self::Decoded(v) => v.fmt(f),
        }
    }
}

impl<'a, T> RepeatedField<'a, T>
where
    &'a T: Deserialize<'a>,
{
    pub fn get(&'a self, index: usize) -> Option<&'a T> {
        match self {
            RepeatedField::Encoded(s) => s.get(index).map(|x| x.ok()).flatten(),
            RepeatedField::Decoded(v) => Some(&v[index]),
        }
    }
}

impl<'a> RepeatedField<'a, u64> {
    pub fn get(&'a self, index: usize) -> Option<u64> {
        match self {
            RepeatedField::Encoded(s) => s.get(index).map(|x| x.ok()).flatten(),
            RepeatedField::Decoded(v) => Some(v[index]),
        }
    }
}

impl<'a, T> RepeatedField<'a, T> {
    pub fn iter(&'a self) -> RepeatedFieldIterator<'a, T> {
        match self {
            Self::Encoded(e) => RepeatedFieldIterator::Encoded(e.iter()),
            Self::Decoded(i) => RepeatedFieldIterator::Decoded(i.iter()),
        }
    }
}

impl<'a, T: Deserialize<'a> + Copy> Iterator for RepeatedFieldIterator<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(si) => {
                let (start, end) = si.next()?;
                Some(T::decode(si.get(start, end)).ok()?)
            }
            Self::Decoded(i) => Some(*i.next()?),
        }
    }
}

impl<'a, T: Copy + Deserialize<'a>> IntoIterator for &'a RepeatedField<'a, T> {
    type Item = T;
    type IntoIter = RepeatedFieldIterator<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            RepeatedField::Encoded(x) => RepeatedFieldIterator::Encoded(x.iter()),
            RepeatedField::Decoded(x) => RepeatedFieldIterator::Decoded(x.iter()),
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

pub enum RepeatedFieldIterator<'a, T> {
    Encoded(EncodedStructIterator<'a>),
    Decoded(std::slice::Iter<'a, T>),
}

pub enum RepeatedString<'a> {
    Encoded(EncodedStruct<'a>),
    Decoded(&'a [String]),
}

pub enum RepeatedStringIterator<'a> {
    Encoded(EncodedStructIterator<'a>),
    Decoded(std::slice::Iter<'a, String>),
}

impl<'a> RepeatedString<'a> {
    pub fn iter(&'a self) -> RepeatedStringIterator<'a> {
        match self {
            Self::Encoded(x) => RepeatedStringIterator::Encoded(x.iter()),
            Self::Decoded(x) => RepeatedStringIterator::Decoded(x.iter()),
        }
    }
}

impl<'a> Iterator for RepeatedStringIterator<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(r) => {
                let (start, end) = r.next()?;
                Deserialize::decode(r.get(start, end)).ok()
            }
            Self::Decoded(s) => s.next().map(|s| s.as_str()),
        }
    }
}

impl<'a> std::fmt::Debug for RepeatedString<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(_) => {
                write!(f, "[")?;
                let mut iter = self.iter();
                let mut next = iter.next();
                while let Some(item) = next {
                    write!(f, "{:?}", item)?;
                    next = iter.next();
                    if next.is_some() {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")
            }
            Self::Decoded(v) => v.fmt(f),
        }
    }
}

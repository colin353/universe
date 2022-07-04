/*
 * DO NOT EDIT THIS FILE
 *
 * It was generated by car's code generator
 *
 */

// Allow dead code, since we're generating structs/accessors
#![allow(dead_code)]

// TODO: remove this
mod test_test;

use car::{
    DeserializeOwned, EncodedStruct, EncodedStructBuilder, RepeatedField, Serialize, RefContainer, RepeatedFieldIterator, RepeatedString
};

#[derive(Clone, Debug, Default)]
struct ZootOwned {
    toot: TootOwned,
    size: Vec<u64>,
    name: String,
}
#[derive(Clone)]
enum Zoot<'a> {
    Encoded(EncodedStruct<'a>),
    DecodedOwned(Box<ZootOwned>),
    DecodedReference(&'a ZootOwned),
}


impl<'a> Default for Zoot<'a> {
    fn default() -> Self {
        Self::DecodedOwned(Box::new(ZootOwned::default()))
    }
}

impl<'a> std::fmt::Debug for Zoot<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Zoot")
         .field("toot", &self.get_toot())
         .field("size", &self.get_size())
         .field("name", &self.get_name())
         .finish()
    }
}

impl Serialize for ZootOwned {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut builder = EncodedStructBuilder::new(writer);
        builder.push(&self.toot)?;
        builder.push(&self.size)?;
        builder.push(&self.name)?;
        builder.finish()
    }
}

impl DeserializeOwned for ZootOwned {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let s = EncodedStruct::new(bytes)?;
        Ok(Self {
            toot: s.get_owned(0).unwrap()?,
            size: s.get_owned(1).unwrap()?,
            name: s.get_owned(2).unwrap()?,
        })
    }
}
impl<'a> DeserializeOwned for Zoot<'a> {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        Ok(Self::DecodedOwned(Box::new(ZootOwned::decode_owned(bytes)?)))
    }
}
enum RepeatedZoot<'a> {
    Encoded(RepeatedField<'a, Zoot<'a>>),
    Decoded(&'a [ZootOwned]),
}

impl<'a> std::fmt::Debug for RepeatedZoot<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(v) => v.fmt(f),
            Self::Decoded(v) => v.fmt(f),
        }
    }
}

enum RepeatedZootIterator<'a> {
    Encoded(RepeatedFieldIterator<'a, Zoot<'a>>),
    Decoded(std::slice::Iter<'a, ZootOwned>),
}

impl<'a> RepeatedZoot<'a> {
    pub fn iter(&'a self) -> RepeatedZootIterator<'a> {
        match self {
            Self::Encoded(r) => RepeatedZootIterator::Encoded(r.iter()),
            Self::Decoded(s) => RepeatedZootIterator::Decoded(s.iter()),
        }
    }
}

impl<'a> Iterator for RepeatedZootIterator<'a> {
    type Item = Zoot<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(it) => match it.next()? {
                RefContainer::Owned(b) => Some(*b),
                _ => None,
            },
            Self::Decoded(it) => Some(Zoot::DecodedReference(it.next()?)),
        }
    }
}

impl<'a> Zoot<'a> {
    pub fn new() -> Self {
        Self::DecodedOwned(Box::new(ZootOwned {
            ..Default::default()
        }))
    }
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::new(bytes)?))
    }
    pub fn to_owned(&self) -> Result<Self, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => Ok(Self::DecodedOwned(t.clone())),
            Self::DecodedReference(t) => Ok(Self::DecodedOwned(Box::new((*t).clone()))),
            Self::Encoded(_) => Ok(Self::DecodedOwned(Box::new(self.clone_owned()?))),
        }
    }

    pub fn clone_owned(&self) -> Result<ZootOwned, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => Ok(t.as_ref().clone()),
            Self::DecodedReference(t) => Ok((*t).clone()),
            Self::Encoded(t) => Ok(ZootOwned {
                toot: t.get_owned(0).unwrap()?,
                size: t.get_owned(1).unwrap()?,
                name: t.get_owned(2).unwrap()?,
            }),
        }
    }
    pub fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => t.encode(writer),
            Self::DecodedReference(t) => t.encode(writer),
            Self::Encoded(t) => t.encode(writer),
        }
    }
    pub fn get_toot(&'a self) -> Toot {
        match self {
            Self::DecodedOwned(x) => Toot::DecodedReference(&x.toot),
            Self::DecodedReference(x) => Toot::DecodedReference(&x.toot),
            Self::Encoded(x) => Toot::Encoded(x.get(0).unwrap().unwrap()),
        }
    }
    pub fn set_toot(&mut self, value: Toot) -> Result<(), std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.set_toot(value)?;
            }
            Self::DecodedOwned(v) => {
                v.toot = value.clone_owned()?;
            }
        }
        Ok(())
    }
    pub fn mut_toot(&mut self) -> Result<&mut TootOwned, std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.mut_toot()
            }
            Self::DecodedOwned(v) => {
                Ok(&mut v.toot)
            }
        }
    }
    pub fn get_size(&'a self) -> RepeatedField<'a, u64> {
        match self {
            Self::DecodedOwned(x) => RepeatedField::DecodedReference(x.size.as_slice()),
            Self::DecodedReference(x) => RepeatedField::DecodedReference(x.size.as_slice()),
            Self::Encoded(x) => RepeatedField::Encoded(x.get(1).unwrap().unwrap()),
        }
    }
    pub fn set_size(&mut self, value: Vec<u64>) -> Result<(), std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.set_size(value)?;
            }
            Self::DecodedOwned(v) => {
                v.size = value;
            }
        }
        Ok(())
    }
    pub fn mut_size(&mut self) -> Result<&mut Vec<u64>, std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.mut_size()
            }
            Self::DecodedOwned(v) => {
                Ok(&mut v.size)
            }
        }
    }
    pub fn get_name(&'a self) -> &str {
        match self {
            Self::DecodedOwned(x) => x.name.as_str(),
            Self::DecodedReference(x) => x.name.as_str(),
            Self::Encoded(x) => x.get(2).unwrap().unwrap(),
        }
    }
    pub fn set_name(&mut self, value: String) -> Result<(), std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.set_name(value)?;
            }
            Self::DecodedOwned(v) => {
                v.name = value;
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default)]
struct TootOwned {
    id: u32,
}
#[derive(Clone)]
enum Toot<'a> {
    Encoded(EncodedStruct<'a>),
    DecodedOwned(Box<TootOwned>),
    DecodedReference(&'a TootOwned),
}


impl<'a> Default for Toot<'a> {
    fn default() -> Self {
        Self::DecodedOwned(Box::new(TootOwned::default()))
    }
}

impl<'a> std::fmt::Debug for Toot<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toot")
         .field("id", &self.get_id())
         .finish()
    }
}

impl Serialize for TootOwned {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut builder = EncodedStructBuilder::new(writer);
        builder.push(&self.id)?;
        builder.finish()
    }
}

impl DeserializeOwned for TootOwned {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let s = EncodedStruct::new(bytes)?;
        Ok(Self {
            id: s.get_owned(0).unwrap()?,
        })
    }
}
impl<'a> DeserializeOwned for Toot<'a> {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        Ok(Self::DecodedOwned(Box::new(TootOwned::decode_owned(bytes)?)))
    }
}
enum RepeatedToot<'a> {
    Encoded(RepeatedField<'a, Toot<'a>>),
    Decoded(&'a [TootOwned]),
}

impl<'a> std::fmt::Debug for RepeatedToot<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(v) => v.fmt(f),
            Self::Decoded(v) => v.fmt(f),
        }
    }
}

enum RepeatedTootIterator<'a> {
    Encoded(RepeatedFieldIterator<'a, Toot<'a>>),
    Decoded(std::slice::Iter<'a, TootOwned>),
}

impl<'a> RepeatedToot<'a> {
    pub fn iter(&'a self) -> RepeatedTootIterator<'a> {
        match self {
            Self::Encoded(r) => RepeatedTootIterator::Encoded(r.iter()),
            Self::Decoded(s) => RepeatedTootIterator::Decoded(s.iter()),
        }
    }
}

impl<'a> Iterator for RepeatedTootIterator<'a> {
    type Item = Toot<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(it) => match it.next()? {
                RefContainer::Owned(b) => Some(*b),
                _ => None,
            },
            Self::Decoded(it) => Some(Toot::DecodedReference(it.next()?)),
        }
    }
}

impl<'a> Toot<'a> {
    pub fn new() -> Self {
        Self::DecodedOwned(Box::new(TootOwned {
            ..Default::default()
        }))
    }
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::new(bytes)?))
    }
    pub fn to_owned(&self) -> Result<Self, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => Ok(Self::DecodedOwned(t.clone())),
            Self::DecodedReference(t) => Ok(Self::DecodedOwned(Box::new((*t).clone()))),
            Self::Encoded(_) => Ok(Self::DecodedOwned(Box::new(self.clone_owned()?))),
        }
    }

    pub fn clone_owned(&self) -> Result<TootOwned, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => Ok(t.as_ref().clone()),
            Self::DecodedReference(t) => Ok((*t).clone()),
            Self::Encoded(t) => Ok(TootOwned {
                id: t.get_owned(0).unwrap()?,
            }),
        }
    }
    pub fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => t.encode(writer),
            Self::DecodedReference(t) => t.encode(writer),
            Self::Encoded(t) => t.encode(writer),
        }
    }
    pub fn get_id(&'a self) -> u32 {
        match self {
            Self::DecodedOwned(x) => x.id,
            Self::DecodedReference(x) => x.id,

            Self::Encoded(x) => x.get(0).unwrap().unwrap(),
        }
    }
    pub fn set_id(&mut self, value: u32) -> Result<(), std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.set_id(value)?;
            }
            Self::DecodedOwned(v) => {
                v.id = value;
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default)]
struct ContainerOwned {
    values: Vec<TootOwned>,
    names: Vec<String>,
}
#[derive(Clone)]
enum Container<'a> {
    Encoded(EncodedStruct<'a>),
    DecodedOwned(Box<ContainerOwned>),
    DecodedReference(&'a ContainerOwned),
}


impl<'a> Default for Container<'a> {
    fn default() -> Self {
        Self::DecodedOwned(Box::new(ContainerOwned::default()))
    }
}

impl<'a> std::fmt::Debug for Container<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
         .field("values", &self.get_values())
         .field("names", &self.get_names())
         .finish()
    }
}

impl Serialize for ContainerOwned {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut builder = EncodedStructBuilder::new(writer);
        builder.push(&self.values)?;
        builder.push(&self.names)?;
        builder.finish()
    }
}

impl DeserializeOwned for ContainerOwned {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let s = EncodedStruct::new(bytes)?;
        Ok(Self {
            values: s.get_owned(0).unwrap()?,
            names: s.get_owned(1).unwrap()?,
        })
    }
}
impl<'a> DeserializeOwned for Container<'a> {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        Ok(Self::DecodedOwned(Box::new(ContainerOwned::decode_owned(bytes)?)))
    }
}
enum RepeatedContainer<'a> {
    Encoded(RepeatedField<'a, Container<'a>>),
    Decoded(&'a [ContainerOwned]),
}

impl<'a> std::fmt::Debug for RepeatedContainer<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(v) => v.fmt(f),
            Self::Decoded(v) => v.fmt(f),
        }
    }
}

enum RepeatedContainerIterator<'a> {
    Encoded(RepeatedFieldIterator<'a, Container<'a>>),
    Decoded(std::slice::Iter<'a, ContainerOwned>),
}

impl<'a> RepeatedContainer<'a> {
    pub fn iter(&'a self) -> RepeatedContainerIterator<'a> {
        match self {
            Self::Encoded(r) => RepeatedContainerIterator::Encoded(r.iter()),
            Self::Decoded(s) => RepeatedContainerIterator::Decoded(s.iter()),
        }
    }
}

impl<'a> Iterator for RepeatedContainerIterator<'a> {
    type Item = Container<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(it) => match it.next()? {
                RefContainer::Owned(b) => Some(*b),
                _ => None,
            },
            Self::Decoded(it) => Some(Container::DecodedReference(it.next()?)),
        }
    }
}

impl<'a> Container<'a> {
    pub fn new() -> Self {
        Self::DecodedOwned(Box::new(ContainerOwned {
            ..Default::default()
        }))
    }
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::new(bytes)?))
    }
    pub fn to_owned(&self) -> Result<Self, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => Ok(Self::DecodedOwned(t.clone())),
            Self::DecodedReference(t) => Ok(Self::DecodedOwned(Box::new((*t).clone()))),
            Self::Encoded(_) => Ok(Self::DecodedOwned(Box::new(self.clone_owned()?))),
        }
    }

    pub fn clone_owned(&self) -> Result<ContainerOwned, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => Ok(t.as_ref().clone()),
            Self::DecodedReference(t) => Ok((*t).clone()),
            Self::Encoded(t) => Ok(ContainerOwned {
                values: t.get_owned(0).unwrap()?,
                names: t.get_owned(1).unwrap()?,
            }),
        }
    }
    pub fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        match self {
            Self::DecodedOwned(t) => t.encode(writer),
            Self::DecodedReference(t) => t.encode(writer),
            Self::Encoded(t) => t.encode(writer),
        }
    }
    pub fn get_values(&'a self) -> RepeatedToot<'a> {
        match self {
            Self::DecodedOwned(x) => RepeatedToot::Decoded(x.values.as_slice()),
            Self::DecodedReference(x) => RepeatedToot::Decoded(&x.values.as_slice()),
            // TODO: remove unwrap
            Self::Encoded(x) => RepeatedToot::Encoded(RepeatedField::Encoded(x.get(0).unwrap().unwrap())),
        }
    }
    pub fn set_values(&mut self, values: Vec<TootOwned>) -> Result<(), std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.set_values(values)?;
            }
            Self::DecodedOwned(v) => {
                v.values = values;
            }
        }
        Ok(())
    }
    pub fn mut_values(&mut self) -> Result<&mut Vec<TootOwned>, std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.mut_values()
            }
            Self::DecodedOwned(v) => {
                Ok(&mut v.values)
            }
        }
    }
    pub fn get_names(&'a self) -> RepeatedString<'a> {
        match self {
            Self::DecodedOwned(x) => RepeatedString::Decoded(x.names.as_slice()),
            Self::DecodedReference(x) => RepeatedString::Decoded(x.names.as_slice()),
            Self::Encoded(x) => RepeatedString::Encoded(x.get(1).unwrap().unwrap()),
        }
    }
    pub fn set_names(&mut self, value: Vec<String>) -> Result<(), std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.set_names(value)?;
            }
            Self::DecodedOwned(v) => {
                v.names = value;
            }
        }
        Ok(())
    }
    pub fn mut_names(&mut self) -> Result<&mut Vec<String>, std::io::Error> {
        match self {
            Self::Encoded(_) | Self::DecodedReference(_) => {
                *self = self.to_owned()?;
                self.mut_names()
            }
            Self::DecodedOwned(v) => {
                Ok(&mut v.names)
            }
        }
    }
}
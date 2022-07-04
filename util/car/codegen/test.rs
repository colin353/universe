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
    Deserialize, DeserializeOwned, EncodedStruct, EncodedStructBuilder, RepeatedField,
    RepeatedFieldIterator, RepeatedString, Serialize,
};

#[derive(Clone, Debug, Default)]
struct Zoot {
    toot: Toot,
    size: Vec<u64>,
    name: String,
}
#[derive(Clone, Copy)]
enum ZootView<'a> {
    Encoded(EncodedStruct<'a>),
    Decoded(&'a Zoot),
}
impl<'a> std::fmt::Debug for ZootView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Zoot")
            .field("toot", &self.get_toot())
            .field("size", &self.get_size())
            .field("name", &self.get_name())
            .finish()
    }
}

impl Serialize for Zoot {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut builder = EncodedStructBuilder::new(writer);
        builder.push(&self.toot)?;
        builder.push(&self.size)?;
        builder.push(&self.name)?;
        builder.finish()
    }
}

impl DeserializeOwned for Zoot {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let s = EncodedStruct::new(bytes)?;
        Ok(Self {
            toot: s.get_owned(0).transpose()?.unwrap_or_default(),
            size: s.get_owned(1).transpose()?.unwrap_or_default(),
            name: s.get_owned(2).transpose()?.unwrap_or_default(),
        })
    }
}
impl<'a> Deserialize<'a> for ZootView<'a> {
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::from_bytes(bytes)?))
    }
}
enum RepeatedZoot<'a> {
    Encoded(RepeatedField<'a, ZootView<'a>>),
    Decoded(&'a [Zoot]),
}

impl<'a> std::fmt::Debug for RepeatedZoot<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(v) => {
                write!(f, "[")?;
                let mut first = true;
                for item in v {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, "]")
            }
            Self::Decoded(v) => v.fmt(f),
        }
    }
}

enum RepeatedZootIterator<'a> {
    Encoded(RepeatedFieldIterator<'a, ZootView<'a>>),
    Decoded(std::slice::Iter<'a, Zoot>),
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
    type Item = ZootView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(it) => it.next(),
            Self::Decoded(it) => Some(ZootView::Decoded(it.next()?)),
        }
    }
}

impl Zoot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, std::io::Error> {
        ZootView::from_bytes(bytes)?.to_owned()
    }

    pub fn as_view<'a>(&'a self) -> ZootView {
        ZootView::Decoded(self)
    }
}
impl<'a> ZootView<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::new(bytes)?))
    }
    pub fn to_owned(&self) -> Result<Zoot, std::io::Error> {
        match self {
            Self::Decoded(t) => Ok((*t).clone()),
            Self::Encoded(t) => Ok(Zoot {
                toot: t.get_owned(0).transpose()?.unwrap_or_default(),
                size: t.get_owned(1).transpose()?.unwrap_or_default(),
                name: t.get_owned(2).transpose()?.unwrap_or_default(),
            }),
        }
    }
    pub fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        match self {
            Self::Decoded(t) => t.encode(writer),
            Self::Encoded(t) => t.encode(writer),
        }
    }
    pub fn get_toot(&'a self) -> TootView<'a> {
        match self {
           Self::Decoded(x) => TootView::Decoded(&x.toot),
            Self::Encoded(x) => TootView::Encoded(x.get(0).transpose().unwrap_or_default().unwrap_or_default()),
        }
    }
    pub fn get_size(&'a self) -> RepeatedField<'a, u64> {
        match self {
            Self::Decoded(x) => RepeatedField::Decoded(x.size.as_slice()),
            Self::Encoded(x) => RepeatedField::Encoded(x.get(1).transpose().unwrap_or_default().unwrap_or_default()),
        }
    }
    pub fn get_name(&'a self) -> &str {
        match self {
            Self::Decoded(x) => x.name.as_str(),
            Self::Encoded(x) => x.get(2).transpose().unwrap_or_default().unwrap_or_default(),
        }
    }
}
#[derive(Clone, Debug, Default)]
struct Toot {
    id: u32,
}
#[derive(Clone, Copy)]
enum TootView<'a> {
    Encoded(EncodedStruct<'a>),
    Decoded(&'a Toot),
}
impl<'a> std::fmt::Debug for TootView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toot")
            .field("id", &self.get_id())
            .finish()
    }
}

impl Serialize for Toot {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut builder = EncodedStructBuilder::new(writer);
        builder.push(&self.id)?;
        builder.finish()
    }
}

impl DeserializeOwned for Toot {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let s = EncodedStruct::new(bytes)?;
        Ok(Self {
            id: s.get_owned(0).transpose()?.unwrap_or_default(),
        })
    }
}
impl<'a> Deserialize<'a> for TootView<'a> {
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::from_bytes(bytes)?))
    }
}
enum RepeatedToot<'a> {
    Encoded(RepeatedField<'a, TootView<'a>>),
    Decoded(&'a [Toot]),
}

impl<'a> std::fmt::Debug for RepeatedToot<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(v) => {
                write!(f, "[")?;
                let mut first = true;
                for item in v {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, "]")
            }
            Self::Decoded(v) => v.fmt(f),
        }
    }
}

enum RepeatedTootIterator<'a> {
    Encoded(RepeatedFieldIterator<'a, TootView<'a>>),
    Decoded(std::slice::Iter<'a, Toot>),
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
    type Item = TootView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(it) => it.next(),
            Self::Decoded(it) => Some(TootView::Decoded(it.next()?)),
        }
    }
}

impl Toot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, std::io::Error> {
        TootView::from_bytes(bytes)?.to_owned()
    }

    pub fn as_view<'a>(&'a self) -> TootView {
        TootView::Decoded(self)
    }
}
impl<'a> TootView<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::new(bytes)?))
    }
    pub fn to_owned(&self) -> Result<Toot, std::io::Error> {
        match self {
            Self::Decoded(t) => Ok((*t).clone()),
            Self::Encoded(t) => Ok(Toot {
                id: t.get_owned(0).transpose()?.unwrap_or_default(),
            }),
        }
    }
    pub fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        match self {
            Self::Decoded(t) => t.encode(writer),
            Self::Encoded(t) => t.encode(writer),
        }
    }
    pub fn get_id(&'a self) -> u32 {
        match self {
            Self::Decoded(x) => x.id,
            Self::Encoded(x) => x.get(0).transpose().unwrap_or_default().unwrap_or_default(),
        }
    }
}
#[derive(Clone, Debug, Default)]
struct Container {
    values: Vec<Toot>,
    names: Vec<String>,
}
#[derive(Clone, Copy)]
enum ContainerView<'a> {
    Encoded(EncodedStruct<'a>),
    Decoded(&'a Container),
}
impl<'a> std::fmt::Debug for ContainerView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Container")
            .field("values", &self.get_values())
            .field("names", &self.get_names())
            .finish()
    }
}

impl Serialize for Container {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut builder = EncodedStructBuilder::new(writer);
        builder.push(&self.values)?;
        builder.push(&self.names)?;
        builder.finish()
    }
}

impl DeserializeOwned for Container {
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let s = EncodedStruct::new(bytes)?;
        Ok(Self {
            values: s.get_owned(0).transpose()?.unwrap_or_default(),
            names: s.get_owned(1).transpose()?.unwrap_or_default(),
        })
    }
}
impl<'a> Deserialize<'a> for ContainerView<'a> {
    fn decode(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::from_bytes(bytes)?))
    }
}
enum RepeatedContainer<'a> {
    Encoded(RepeatedField<'a, ContainerView<'a>>),
    Decoded(&'a [Container]),
}

impl<'a> std::fmt::Debug for RepeatedContainer<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoded(v) => {
                write!(f, "[")?;
                let mut first = true;
                for item in v {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, "]")
            }
            Self::Decoded(v) => v.fmt(f),
        }
    }
}

enum RepeatedContainerIterator<'a> {
    Encoded(RepeatedFieldIterator<'a, ContainerView<'a>>),
    Decoded(std::slice::Iter<'a, Container>),
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
    type Item = ContainerView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Encoded(it) => it.next(),
            Self::Decoded(it) => Some(ContainerView::Decoded(it.next()?)),
        }
    }
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, std::io::Error> {
        ContainerView::from_bytes(bytes)?.to_owned()
    }

    pub fn as_view<'a>(&'a self) -> ContainerView {
        ContainerView::Decoded(self)
    }
}
impl<'a> ContainerView<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        Ok(Self::Encoded(EncodedStruct::new(bytes)?))
    }
    pub fn to_owned(&self) -> Result<Container, std::io::Error> {
        match self {
            Self::Decoded(t) => Ok((*t).clone()),
            Self::Encoded(t) => Ok(Container {
                values: t.get_owned(0).transpose()?.unwrap_or_default(),
                names: t.get_owned(1).transpose()?.unwrap_or_default(),
            }),
        }
    }
    pub fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        match self {
            Self::Decoded(t) => t.encode(writer),
            Self::Encoded(t) => t.encode(writer),
        }
    }
    pub fn get_values(&'a self) -> RepeatedToot<'a> {
        match self {
            Self::Decoded(x) => RepeatedToot::Decoded(&x.values.as_slice()),
            Self::Encoded(x) => RepeatedToot::Encoded(RepeatedField::Encoded(x.get(0).transpose().unwrap_or_default().unwrap_or_default())),
        }
    }
    pub fn get_names(&'a self) -> RepeatedString<'a> {
        match self {
            Self::Decoded(x) => RepeatedString::Decoded(x.names.as_slice()),
            Self::Encoded(x) => RepeatedString::Encoded(x.get(1).transpose().unwrap_or_default().unwrap_or_default()),
        }
    }
}

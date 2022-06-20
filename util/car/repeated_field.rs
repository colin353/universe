use crate::pack;
use crate::varint;
use crate::Serializable;

pub struct RepeatedField<'a> {
    data: &'a [u8],
    fields_index: pack::Pack<'a>,
    empty: bool,
}

pub struct RepeatedFieldBuilder<W: std::io::Write> {
    sizes: Vec<u32>,
    writer: W,
}

impl<W: std::io::Write> RepeatedFieldBuilder<W> {
    pub fn new(writer: W) -> Self {
        Self {
            sizes: Vec::new(),
            writer,
        }
    }

    pub fn push<'a, T: Serializable<'a>>(&mut self, value: T) -> Result<(), std::io::Error> {
        let length = value.encode(&mut self.writer)?;
        self.sizes.push(length as u32);
        Ok(())
    }

    pub fn finish(mut self) -> Result<usize, std::io::Error> {
        if self.sizes.is_empty() {
            return Ok(0);
        }

        let mut data_size = 0;
        let mut pack = pack::PackBuilder::new(&mut self.writer);
        for size in &self.sizes[0..self.sizes.len() - 1] {
            pack.push(*size)?;
            data_size += *size;
        }

        let pack_size = pack.finish()?;

        // Write the footer, which is the number of encoded elements
        let footer_size = varint::encode_reverse_varint((pack_size + 1) as u32, &mut self.writer)?;

        Ok(data_size as usize + pack_size + footer_size)
    }
}

impl<'a> RepeatedField<'a> {
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
        let data_length = data.len() - pack_size - footer_size;
        Ok(Self {
            empty: false,
            data: &data[0..data_length],
            fields_index: pack::Pack::new(&data[data_length..data.len() - footer_size])?,
        })
    }

    pub fn get<T: Serializable<'a>>(&'a self, idx: usize) -> Option<Result<T, std::io::Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repeated_field() {
        let mut buf = Vec::new();
        let mut b = RepeatedFieldBuilder::new(&mut buf);
        b.push("asdf").unwrap();
        b.push("fdsa").unwrap();
        b.finish().unwrap();

        let rf = RepeatedField::new(&buf).unwrap();
        assert_eq!(rf.len(), 2);
        assert_eq!(rf.is_empty(), false);
        assert_eq!(rf.get::<&str>(0).unwrap().unwrap(), "asdf");
        assert_eq!(rf.get::<&str>(1).unwrap().unwrap(), "fdsa");
        assert_eq!(rf.get::<&str>(2).is_none(), true);
    }

    #[test]
    fn test_empty_repeated_field() {
        let mut buf = Vec::new();
        let mut b = RepeatedFieldBuilder::new(&mut buf);
        b.finish().unwrap();

        let rf = RepeatedField::new(&buf).unwrap();
        assert_eq!(rf.len(), 0);
        assert_eq!(rf.is_empty(), true);
        assert_eq!(rf.get::<&str>(0).is_none(), true);
    }

    #[test]
    fn test_repeated_field_one_empty_item() {
        let mut buf = Vec::new();
        let mut b = RepeatedFieldBuilder::new(&mut buf);
        b.push("").unwrap(); // encoded size: zero
        b.finish().unwrap();

        let rf = RepeatedField::new(&buf).unwrap();
        assert_eq!(rf.len(), 1);
        assert_eq!(rf.is_empty(), false);
        assert_eq!(rf.get::<&str>(0).unwrap().unwrap(), "");
    }
}

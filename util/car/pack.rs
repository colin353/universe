use crate::varint;

pub struct Pack<'a> {
    data: &'a [u8],
    offsets: &'a [u32],
    offsets_index: &'a [u32],
}

impl<'a> Pack<'a> {
    // The pack structure is:
    //
    // [ data u8 ... ] [ offsets u32 ... ] [ offsets index u32 ... ] [ footer ]
    //
    // The footer encodes the number of offsets.
    // The offsets index is always 1/16th as long as the data bytes.
    pub fn new(bytes: &'a [u8]) -> Result<Self, std::io::Error> {
        let (num_values, footer_size) = varint::decode_reverse_varint(bytes);
        let offset_index_start = bytes.len() - footer_size - (num_values / 16) * 4;
        let offset_index_end = bytes.len() - footer_size;

        Ok(Self {
            data: &bytes[0..num_values],
            offsets: unsafe {
                std::slice::from_raw_parts(
                    bytes[num_values..offset_index_start].as_ptr() as *const u32,
                    bytes.len() - num_values - (num_values / 16) * 4 - footer_size,
                )
            },
            offsets_index: unsafe {
                std::slice::from_raw_parts(
                    bytes[offset_index_start..offset_index_end].as_ptr() as *const u32,
                    num_values / 4,
                )
            },
        })
    }

    pub fn get(&self, idx: usize) -> Option<u32> {
        if idx >= self.data.len() {
            return None;
        }

        let block_start = idx & 0xFFFFFFF0;
        let block_end = std::cmp::min(self.data.len(), block_start + 16);
        let block: u128 = read_u128(&self.data[block_start..block_end]);

        // Figure out the offset at the start of this block
        let block_offset_idx = idx / 16;
        let block_offset_position = if block_offset_idx > 0 {
            self.offsets_index[block_offset_idx - 1]
        } else {
            0
        };

        // Count any additional offsets marked during this block
        let right_shift = (15 - (idx % 16)) * 8;
        let mask = 0x80808080808080808080808080808080 >> right_shift;

        let extra_offset_count = (block & mask).count_ones();

        // Find the offset
        let offset_position = (block_offset_position + extra_offset_count) as usize;
        let offset = if offset_position > 0 {
            self.offsets[offset_position - 1]
        } else {
            0
        };

        // Add up all the deltas between the last offset and this one
        let last_overflow = if block & mask > 0 {
            block_start + 16 - ((block & mask).leading_zeros() / 8) as usize
        } else {
            block_start
        };

        let delta: u32 = self.data[last_overflow..idx + 1]
            .iter()
            .map(|x| *x as u32)
            .sum();

        Some(offset + delta)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

pub struct PackBuilder<W: std::io::Write> {
    count: usize,
    total: u32,
    writer: W,
    offsets: Vec<u32>,
    offset_index: Vec<u32>,
}

impl<W: std::io::Write> PackBuilder<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            count: 0,
            total: 0,
            offsets: Vec::new(),
            offset_index: Vec::new(),
        }
    }

    pub fn push(&mut self, delta: u32) -> Result<(), std::io::Error> {
        self.count += 1;
        self.total += delta;

        if self.count % 16 == 0 {
            self.offsets.push(self.total);
            self.offset_index.push(self.offsets.len() as u32);
            self.writer.write_all(&[128])?;
        } else if delta >= 128 {
            self.offsets.push(self.total);
            self.writer.write_all(&[128])?;
        } else {
            self.writer.write_all(&[delta as u8])?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<usize, std::io::Error> {
        let num_offsets = self.offsets.len();
        for offset in self.offsets {
            self.writer.write_all(&offset.to_le_bytes())?;
        }
        for value in self.offset_index {
            self.writer.write_all(&value.to_le_bytes())?;
        }
        let footer_size = varint::encode_reverse_varint(self.count as u32, &mut self.writer)?;
        Ok(self.count + (self.count / 4) + num_offsets * 4 + footer_size)
    }
}

#[inline(always)]
fn u128_from_bytes(bytes: &[u8], n: usize) -> u128 {
    let mut tmp = [0; 16];
    tmp[..n].copy_from_slice(&bytes[..n]);
    u128::from_le_bytes(tmp)
}

fn read_u128(bytes: &[u8]) -> u128 {
    match bytes.len() {
        0 => u128_from_bytes(bytes, 0),
        1 => u128_from_bytes(bytes, 1),
        2 => u128_from_bytes(bytes, 2),
        3 => u128_from_bytes(bytes, 3),
        4 => u128_from_bytes(bytes, 4),
        5 => u128_from_bytes(bytes, 5),
        6 => u128_from_bytes(bytes, 6),
        7 => u128_from_bytes(bytes, 7),
        8 => u128_from_bytes(bytes, 8),
        9 => u128_from_bytes(bytes, 9),
        10 => u128_from_bytes(bytes, 10),
        11 => u128_from_bytes(bytes, 11),
        12 => u128_from_bytes(bytes, 12),
        13 => u128_from_bytes(bytes, 13),
        14 => u128_from_bytes(bytes, 14),
        15 => u128_from_bytes(bytes, 15),
        _ => u128_from_bytes(bytes, 16),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_builder() {
        // Empty pack should be zero bytes
        let mut buf = Vec::new();
        let p = PackBuilder::new(&mut buf);
        p.finish().unwrap();
        assert_eq!(buf.len(), 0);

        // A single small item in a pack should be just one byte + footer
        let mut buf = Vec::new();
        let mut p = PackBuilder::new(&mut buf);
        p.push(15).unwrap();
        p.finish().unwrap();
        assert_eq!(&buf, &[15, 1]);

        // Inserting more items should trigger an offset
        let mut buf = Vec::new();
        let mut p = PackBuilder::new(&mut buf);
        for i in 0..20 {
            p.push(i).unwrap();
        }
        p.finish().unwrap();
        assert_eq!(
            &buf,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 128, 16, 17, 18, 19, // data
                120, 0, 0, 0, // first offset
                1, 0, 0, 0,  // number of offsets in first block
                20  // footer - number of items
            ]
        );

        // Inserting large items should trigger early offset
        let mut buf = Vec::new();
        let mut p = PackBuilder::new(&mut buf);
        p.push(1000).unwrap();
        p.push(2000).unwrap();
        p.finish().unwrap();
        assert_eq!(
            &buf,
            &[
                128, 128, // data
                232, 3, 0, 0, // first offset
                184, 11, 0, 0, // second offset
                2  // footer - number of items
            ]
        );
    }

    #[test]
    fn test_pack_decoder() {
        // Empty pack
        let mut buf = Vec::new();
        let b = PackBuilder::new(&mut buf);
        b.finish().unwrap();
        let p = Pack::new(&buf).unwrap();
        assert_eq!(p.len(), 0);
        assert_eq!(p.get(0), None);

        // Simple pack
        let mut buf = Vec::new();
        let mut b = PackBuilder::new(&mut buf);
        b.push(1).unwrap();
        b.push(1).unwrap();
        b.push(1).unwrap();
        b.finish().unwrap();
        let p = Pack::new(&buf).unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p.get(0), Some(1));
        assert_eq!(p.get(1), Some(2));
        assert_eq!(p.get(2), Some(3));
        assert_eq!(p.get(3), None);

        // Larger pack
        let mut buf = Vec::new();
        let mut b = PackBuilder::new(&mut buf);
        for i in 0..20 {
            b.push(i).unwrap();
        }
        b.finish().unwrap();
        let p = Pack::new(&buf).unwrap();
        assert_eq!(p.len(), 20);
        let mut sum = 0;
        for i in 0..20 {
            sum += i as u32;
            assert_eq!(p.get(i), Some(sum));
        }
        assert_eq!(p.get(20), None);

        // Pack with big numbers
        let mut buf = Vec::new();
        let mut b = PackBuilder::new(&mut buf);
        b.push(1000).unwrap();
        b.push(2000).unwrap();
        b.finish().unwrap();
        let p = Pack::new(&buf).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p.get(0), Some(1000));
        assert_eq!(p.get(1), Some(3000));
        assert_eq!(p.get(2), None);

        // Pack with lots of big numbers
        let mut buf = Vec::new();
        let mut b = PackBuilder::new(&mut buf);
        for i in 0..20 {
            b.push(1000 * i).unwrap();
        }
        b.finish().unwrap();
        let p = Pack::new(&buf).unwrap();
        assert_eq!(p.len(), 20);
        assert_eq!(p.get(0), Some(0));
        assert_eq!(p.get(1), Some(1000));
        assert_eq!(p.get(10), Some(55000));
        assert_eq!(p.get(15), Some(120000));
        assert_eq!(p.get(25), None);
    }
}

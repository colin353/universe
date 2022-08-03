use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const KIB: usize = 2 << 10;
const MIB: usize = 2 << 20;

pub struct BloomFilterBuilder {
    bytes: Vec<u8>,
    record_count: u64,
}

pub struct BloomFilter<'a> {
    bytes: &'a [u8],
}

impl BloomFilterBuilder {
    pub fn empty() -> Self {
        Self::custom(0)
    }

    pub fn tiny() -> Self {
        Self::custom(100 * KIB)
    }

    pub fn small() -> Self {
        Self::custom(1 * MIB)
    }

    pub fn large() -> Self {
        Self::custom(50 * MIB)
    }

    pub fn custom(bytes: usize) -> Self {
        Self {
            bytes: vec![0; bytes],
            record_count: 0,
        }
    }

    fn set_bit(&mut self, h: u64) {
        if self.bytes.is_empty() {
            return;
        }
        let byte_idx = (h >> 3) % (self.bytes.len() as u64);
        let bit_mask = 0b01 << (h & 0b111);
        self.bytes[byte_idx as usize] |= bit_mask;
    }

    fn check_bit(&self, h: u64) -> bool {
        let byte_idx = (h >> 3) % (self.bytes.len() as u64);
        let bit_mask = 0b01 << (h & 0b111);
        self.bytes[byte_idx as usize] & bit_mask > 0
    }

    pub fn insert(&mut self, key: &str) {
        self.set_bit(BloomFilter::hash(0, key));
        self.set_bit(BloomFilter::hash(1, key));
    }

    pub fn contains(&self, key: &str) -> bool {
        if self.bytes.is_empty() {
            return true;
        }
        self.check_bit(BloomFilter::hash(0, key)) && self.check_bit(BloomFilter::hash(1, key))
    }

    pub fn optimize(mut self) -> Vec<u8> {
        // size = ceil(count * log(0.1)) / (log(1 / pow(2, log(2))))
        let x = (self.record_count as f64 * 0.1_f64.ln()).ceil();
        let y = 2_f64.powf(-(2_f64.ln())).ln();
        let optimal_size = std::cmp::max(128, (x / y) as usize);

        let mut target_size = self.bytes.len();
        let mut should_compress = false;
        while target_size > 2 * optimal_size {
            should_compress = true;
            target_size /= 2;
        }

        if should_compress {
            for i in target_size..self.bytes.len() {
                self.bytes[i % target_size] |= self.bytes[i];
            }
            self.bytes.truncate(target_size);
        }

        self.bytes
    }
}

impl<'a> BloomFilter<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn hash(mode: usize, key: &str) -> u64 {
        let mut s = DefaultHasher::new();
        mode.hash(&mut s);
        key.hash(&mut s);
        s.finish()
    }

    fn check_bit(&self, h: u64) -> bool {
        let byte_idx = (h >> 3) % (self.bytes.len() as u64);
        let bit_mask = 0b01 << (h & 0b111);
        self.bytes[byte_idx as usize] & bit_mask > 0
    }

    pub fn contains(&self, key: &str) -> bool {
        if self.bytes.is_empty() {
            return true;
        }
        self.check_bit(Self::hash(0, key)) && self.check_bit(Self::hash(1, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let mut b = BloomFilterBuilder::empty();
        assert!(b.contains("asdf"));
        assert!(b.contains("fdsa"));
        b.insert("asdf");
        assert!(b.contains("asdf"));
        assert!(b.contains("fdsa"));

        let optimized = b.optimize();
        let f = BloomFilter::from_bytes(&optimized);
        assert!(f.contains("asdf"));
        assert!(f.contains("fdsa"));
    }

    #[test]
    fn test_bloom_filter() {
        let mut b = BloomFilterBuilder::small();
        assert!(!b.contains("asdf"));
        assert!(!b.contains("fdsa"));
        b.insert("asdf");
        assert!(b.contains("asdf"));
        assert!(!b.contains("fdsa"));

        assert_eq!(b.bytes.iter().map(|b| b.count_ones()).sum::<u32>(), 2);

        let optimized = b.optimize();
        assert_eq!(optimized.iter().map(|b| b.count_ones()).sum::<u32>(), 2);

        let f = BloomFilter::from_bytes(&optimized);
        assert!(f.contains("asdf"));
        assert!(!f.contains("fdsa"));
    }
}

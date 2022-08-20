//! Tiny sha-256 implementation in pure rust.
#![warn(missing_docs)]
#![allow(non_snake_case)]
#![no_std]

macro_rules! overflowing_add {
    ($b: expr, $($a: expr),+) => {
        $b $(
            .overflowing_add($a).0
        )+
    }
}

/// initial state of hasher.
const INIT: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// first 32 bits of the fractional parts of the cube roots of the first 64 primes `2...311`.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// `SHA-256` hasher.  Does all the hashing.
pub struct Sha256 {
    /// size of current data fold.
    size: usize,
    /// number of times `data` was
    /// filled & processed.
    reps: usize,
    /// input data buffer.
    data: [u8; 64],
    /// final data buffer.
    buff: [u32; 8],
}

impl Sha256 {
    /// Instantiate a new hasher.
    pub fn new() -> Self {
        Sha256 {
            data: [0; 64],
            size: 0,
            reps: 0,
            buff: INIT,
        }
    }

    /// Absorb some bytes into the hasher.
    pub fn absorb(&mut self, bytes: &[u8]) {
        for byte in bytes.iter() {
            self.absorb_byte(*byte);
        }
    }

    /// Absorb a single byte into the hasher.
    #[inline]
    fn absorb_byte(&mut self, byte: u8) {
        self.data[self.size] = byte;
        self.size += 1;
        if self.size == 64 {
            self.process();
            self.data = [0; 64];
            self.reps += 1;
            self.size = 0;
        }
    }

    /// process a filled data block.
    #[inline]
    fn process(&mut self) {
        let mut w = [0u32; 64];
        for (dest, chunk) in w.iter_mut().zip(self.data.chunks(4)) {
            for byte in chunk.iter() {
                *dest <<= 8;
                *dest |= *byte as u32;
            }
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ w[i - 15] >> 3;
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ w[i - 2] >> 10;
            w[i] = overflowing_add!(w[i - 16], s0, w[i - 7], s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) = (
            self.buff[0],
            self.buff[1],
            self.buff[2],
            self.buff[3],
            self.buff[4],
            self.buff[5],
            self.buff[6],
            self.buff[7],
        );
        for i in 0..64 {
            let S1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = overflowing_add!(h, S1, ch, K[i], w[i]);
            let S0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = overflowing_add!(S0, maj);
            h = g;
            g = f;
            f = e;
            e = overflowing_add!(d, temp1);
            d = c;
            c = b;
            b = a;
            a = overflowing_add!(temp1, temp2);
        }
        self.buff[0] = overflowing_add!(self.buff[0], a);
        self.buff[1] = overflowing_add!(self.buff[1], b);
        self.buff[2] = overflowing_add!(self.buff[2], c);
        self.buff[3] = overflowing_add!(self.buff[3], d);
        self.buff[4] = overflowing_add!(self.buff[4], e);
        self.buff[5] = overflowing_add!(self.buff[5], f);
        self.buff[6] = overflowing_add!(self.buff[6], g);
        self.buff[7] = overflowing_add!(self.buff[7], h);
    }

    /// Finish the hashing process.  Consumes the
    /// hasher and returns the final result.
    pub fn finish(mut self) -> [u8; 32] {
        let L = (self.size * 8) + (self.reps * 512);
        let rem = (L + 64 + 8) % 512;
        let k = if rem == 0 { 0 } else { 512 - rem };
        self.absorb(&[0x80]);
        for _ in 0..(k / 8) {
            self.absorb_byte(0);
        }
        let mut lbuf = [0u8; 8];
        for (i, byte) in lbuf.iter_mut().enumerate() {
            *byte = (L >> (56 - (i * 8))) as u8;
        }
        self.absorb(&lbuf);
        debug_assert!(self.size == 0);
        let mut rslt = [0u8; 32];
        for (bytes, value) in rslt.chunks_mut(4).zip(self.buff.iter()) {
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = (value >> (24 - (i * 8))) as u8;
            }
        }
        rslt
    }
}

#[cfg(test)]
mod test {
    use ::Sha256;

    #[test]
    fn empty() {
        let exp: [u8; 32] = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        let mut hasher = Sha256::new();
        hasher.absorb(&[]);
        let fin = hasher.finish();
        assert_eq!(exp, fin);
    }

    #[test]
    fn hello() {
        let exp: [u8; 32] = [
            0xb9, 0x4d, 0x27, 0xb9, 0x93, 0x4d, 0x3e, 0x08, 0xa5, 0x2e, 0x52, 0xd7, 0xda, 0x7d,
            0xab, 0xfa, 0xc4, 0x84, 0xef, 0xe3, 0x7a, 0x53, 0x80, 0xee, 0x90, 0x88, 0xf7, 0xac,
            0xe2, 0xef, 0xcd, 0xe9,
        ];
        let mut hasher = Sha256::new();
        hasher.absorb("hello world".as_bytes());
        let fin = hasher.finish();
        assert_eq!(exp, fin);
    }

    #[test]
    fn green_eggs() {
        let exp: [u8; 32] = [
            0xa1, 0x13, 0xa9, 0x85, 0x4a, 0xb7, 0x1a, 0x49, 0x14, 0xe2, 0x19, 0xe1, 0x81, 0xca,
            0x8b, 0xfd, 0x48, 0xd7, 0xd6, 0x5b, 0xdb, 0x1c, 0x3c, 0xb1, 0xba, 0xd6, 0x23, 0x5c,
            0x5f, 0x1a, 0xcf, 0x23,
        ];
        let mut hasher = Sha256::new();
        hasher.absorb("green eggs and ham".as_bytes());
        let fin = hasher.finish();
        assert_eq!(exp, fin);
    }

    #[test]
    fn apples() {
        let exp: [u8; 32] = [
            0x8a, 0x2f, 0x37, 0x66, 0xed, 0xc0, 0x22, 0x81, 0xf8, 0x48, 0xcc, 0x80, 0xb9, 0xdf,
            0xe9, 0xef, 0xe1, 0x57, 0xd8, 0xa0, 0xa7, 0xb8, 0xbc, 0x3d, 0xab, 0x6b, 0x8b, 0xdb,
            0x65, 0x1e, 0x2b, 0x9c,
        ];
        let mut hasher = Sha256::new();
        for _ in 0..32 {
            hasher.absorb("apples banannas carrots grapes ".as_bytes());
        }
        let fin = hasher.finish();
        assert_eq!(exp, fin);
    }
}

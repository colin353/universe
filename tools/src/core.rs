use std::collections::hash_map::DefaultHasher;
use std::fmt::Write;
use std::hash::Hasher;
use std::io::Read;

mod lcs;
pub mod patience;

// TODO: get a real hash like SHA256
pub fn hash_file(path: &std::path::Path) -> std::io::Result<[u8; 32]> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0_u8; 8192];
    let mut h = DefaultHasher::new();

    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.write(&buf);
    }

    let bytes = h.finish().to_le_bytes();
    Ok([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ])
}

pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut h = DefaultHasher::new();
    h.write(&bytes);
    let bytes = h.finish().to_le_bytes();
    [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]
}

pub fn fmt_sha(sha: &[u8]) -> String {
    let mut out = String::new();
    for &byte in sha {
        write!(&mut out, "{:x} ", byte).unwrap();
    }
    out
}

pub fn parse_sha(sha: &str) -> std::io::Result<[u8; 32]> {
    let mut out = [0_u8; 32];
    for i in (0..std::cmp::min(32, sha.len())).step_by(2) {
        out[i] = u8::from_str_radix(&sha[i..i + 2], 16).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse as SHA")
        })?;
    }
    Ok(out)
}

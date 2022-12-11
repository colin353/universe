use std::fmt::Write;
use std::io::{Read, Write as _};

// The size of a diff when LZ4 compression is enabled
pub const COMPRESSION_THRESHOLD: usize = 128;
const COMPRESSION_LEVEL: u32 = 1;

mod lcs;
pub mod patience;
pub mod render;

pub fn timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

pub fn compress_rw<R: std::io::Read, W: std::io::Write>(
    reader: &mut R,
    writer: W,
) -> std::io::Result<()> {
    let mut encoder = lz4::EncoderBuilder::new()
        .level(COMPRESSION_LEVEL)
        .build(writer)
        .expect("constructing encoder should be infallible!");

    std::io::copy(reader, &mut encoder)?;
    let (_, result) = encoder.finish();
    result
}

pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = lz4::EncoderBuilder::new()
        .level(COMPRESSION_LEVEL)
        .build(Vec::new())
        .expect("constructing encoder should be infallible!");
    encoder
        .write_all(data)
        .expect("writing to encoder should be infallible!");
    let (output, result) = encoder.finish();
    result.expect("writing to encoder should be infallible!");
    output
}

pub fn decompress(method: service::CompressionKind, data: &[u8]) -> std::io::Result<Vec<u8>> {
    match method {
        service::CompressionKind::None => Ok(data.to_owned()),
        service::CompressionKind::LZ4 => {
            let mut out = Vec::new();
            let mut decoder = lz4::Decoder::new(data)?;
            decoder.read_to_end(&mut out)?;
            Ok(out)
        }
    }
}

pub fn hash_file(path: &std::path::Path) -> std::io::Result<[u8; 32]> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0_u8; 8192];
    let mut h = sha256::Sha256::new();

    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.absorb(&buf);
    }

    Ok(h.finish())
}

pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut h = sha256::Sha256::new();
    h.absorb(&bytes);
    h.finish()
}

pub fn diff(original: &[u8], modified: &[u8]) -> Vec<service::ByteDiff> {
    let (original_s, modified_s) =
        match (std::str::from_utf8(original), std::str::from_utf8(modified)) {
            (Ok(o), Ok(m)) => (o, m),
            _ => {
                let data = compress(modified);
                // If the files are binary, don't compute a diff, just register a deletion and an
                // addition.
                return vec![
                    service::ByteDiff {
                        start: 0,
                        end: original.len() as u32,
                        kind: service::DiffKind::Removed,
                        data: vec![],
                        compression: service::CompressionKind::None,
                    },
                    service::ByteDiff {
                        start: 0,
                        end: modified.len() as u32,
                        kind: service::DiffKind::Added,
                        data,
                        compression: service::CompressionKind::LZ4,
                    },
                ];
            }
        };

    let mut original_lines = Vec::new();
    let mut pos = 0;
    for (idx, _) in original_s.match_indices('\n') {
        original_lines.push(&original[pos..idx + 1]);
        pos = idx + 1;
    }
    if !&original[pos..].is_empty() {
        original_lines.push(&original[pos..]);
    }

    let mut modified_lines = Vec::new();
    let mut pos = 0;
    for (idx, _) in modified_s.match_indices('\n') {
        modified_lines.push(&modified[pos..idx + 1]);
        pos = idx + 1;
    }
    if !&modified[pos..].is_empty() {
        modified_lines.push(&modified[pos..]);
    }

    let mut out = Vec::new();
    let mut left_pos = 0_u32;
    let mut right_pos = 0_u32;
    let diffs = patience::patience_diff(&original_lines, &modified_lines);
    let mut diff_iter = diffs.iter().peekable();
    while let Some(diff) = diff_iter.next() {
        match diff {
            patience::DiffComponent::Unchanged(left, right) => {
                left_pos += left.len() as u32;
                right_pos += right.len() as u32;
            }
            patience::DiffComponent::Insertion(right) => {
                let start = right_pos as usize;
                let left_start = left_pos;
                let mut kind = service::DiffKind::Added;
                right_pos += right.len() as u32;
                loop {
                    if let Some(patience::DiffComponent::Insertion(right)) = diff_iter.peek() {
                        right_pos += right.len() as u32;
                        diff_iter.next();
                    } else if let Some(patience::DiffComponent::Deletion(right)) = diff_iter.peek()
                    {
                        kind = service::DiffKind::Modified;
                        left_pos += right.len() as u32;
                        diff_iter.next();
                    } else {
                        break;
                    }
                }

                let diff_data = &modified[start..right_pos as usize];
                let (data, compression) = if diff_data.len() < COMPRESSION_THRESHOLD {
                    (diff_data.to_owned(), service::CompressionKind::None)
                } else {
                    (compress(diff_data), service::CompressionKind::LZ4)
                };

                out.push(service::ByteDiff {
                    start: left_start,
                    end: left_pos,
                    kind,
                    data,
                    compression,
                });
            }
            patience::DiffComponent::Deletion(left) => {
                let start = left_pos;
                left_pos += left.len() as u32;
                while let Some(patience::DiffComponent::Insertion(left)) = diff_iter.peek() {
                    left_pos += left.len() as u32;
                    diff_iter.next();
                }
                out.push(service::ByteDiff {
                    start: start,
                    end: left_pos,
                    kind: service::DiffKind::Removed,
                    data: vec![],
                    compression: service::CompressionKind::None,
                });
            }
        }
    }
    out
}

// Compute the difference between two patches
pub fn patch_diff(
    prev: &[service::FileDiff],
    curr: &[service::FileDiff],
) -> Vec<service::FileDiff> {
    let mut prev_iter = prev.iter().enumerate().map(|(idx, _)| idx).peekable();
    let mut curr_iter = curr.iter().enumerate().map(|(idx, _)| idx).peekable();

    let joined_iter = std::iter::from_fn(move || {
        match (prev_iter.peek(), curr_iter.peek()) {
            (Some(&p), Some(&c)) => {
                // Both changes affect the same files. Must diff the diffs!
                if prev[p].path == curr[c].path && prev[p].is_dir == curr[c].is_dir {
                    curr_iter.next();
                    prev_iter.next();
                    return Some((Some(p), Some(c)));
                }

                if prev[p].path < curr[c].path {
                    curr_iter.next();
                    Some((Some(p), None))
                } else {
                    prev_iter.next();
                    Some((None, Some(c)))
                }
            }
            (None, Some(&c)) => {
                curr_iter.next();
                Some((None, Some(c)))
            }
            (Some(&p), None) => {
                prev_iter.next();
                Some((Some(p), None))
            }
            (None, None) => None,
        }
    });

    let mut out = Vec::new();
    for r in joined_iter {
        match r {
            (Some(p), Some(c)) => {
                let c = &curr[c];
                let p = &prev[p];
                // Both changes affect the same files. Must diff the diffs!
                match (p.kind, c.kind) {
                    (service::DiffKind::Added, service::DiffKind::Added)
                    | (service::DiffKind::Modified, service::DiffKind::Modified) => {
                        if p.differences != c.differences {
                            // Both were added/modified. So we need to combine their byte diffs
                            let mut change = c.clone();
                            change.kind = service::DiffKind::Modified;
                            change.differences = patch_diff_file(&p.differences, &c.differences);
                            out.push(change);
                        }
                    }
                    (service::DiffKind::Removed, service::DiffKind::Modified) => {
                        // Was deleted, now modified. That is equivalent to an add.
                        let mut change = c.clone();
                        change.kind = service::DiffKind::Added;
                        // Indicate that the file was reverted, before any further
                        // modifications.
                        change.differences.insert(
                            0,
                            service::ByteDiff {
                                start: 0,
                                kind: service::DiffKind::Reverted,
                                ..Default::default()
                            },
                        );
                        out.push(change);
                    }
                    (service::DiffKind::Modified, service::DiffKind::Removed) => {
                        // Was modified, now deleted. Deleted overrides modified
                        out.push(c.clone());
                    }
                    // Any other combination is nonsensical (e.g. Modified --> Add) because it
                    // implies that the basis changed.
                    _ => (),
                }
            }
            (None, Some(c)) => {
                let c = &curr[c];
                // Diff exists in current, but not previously. Therefore it should just directly be
                // appended to the output diff.
                out.push(c.clone());
            }
            (Some(p), None) => {
                let p = &prev[p];
                // Diff existed before, but not anymore. That means it was reverted. We record this
                // with a special "reverted" diff record
                out.push(service::FileDiff {
                    path: p.path.clone(),
                    kind: service::DiffKind::Reverted,
                    is_dir: p.is_dir,
                    differences: vec![],
                });
            }
            (None, None) => break,
        }
    }
    out
}

pub fn patch_diff_file(
    prev: &[service::ByteDiff],
    curr: &[service::ByteDiff],
) -> Vec<service::ByteDiff> {
    // TODO: diff the files via their byte differences?
    vec![]
}

pub fn fmt_sha(sha: &[u8]) -> String {
    let mut out = String::new();
    for &byte in sha {
        write!(&mut out, "{:02x}", byte).unwrap();
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

pub fn fmt_basis(basis: service::BasisView) -> String {
    if basis.get_index() == 0 && basis.get_change() == 0 {
        return format!(
            "{}/{}/{}",
            basis.get_host(),
            basis.get_owner(),
            basis.get_name()
        );
    }

    if basis.get_change() == 0 {
        return format!(
            "{}/{}/{}/{}",
            basis.get_host(),
            basis.get_owner(),
            basis.get_name(),
            basis.get_index()
        );
    }

    return format!(
        "{}/{}/{}/change/{}/{}",
        basis.get_host(),
        basis.get_owner(),
        basis.get_name(),
        basis.get_change(),
        basis.get_index()
    );
}

pub fn parse_basis(basis: &str) -> std::io::Result<service::Basis> {
    let mut components = basis.split("/");
    let host = components.next().expect("split must have one component");
    if host.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "host cannot be empty",
        ));
    }

    let owner = match components.next() {
        Some(c) => c,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid basis (must be of the form <host>[:port]/<owner>/<name>[/<index>])",
            ));
        }
    };

    let name = match components.next() {
        Some(c) => c,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid basis (must be of the form <host>[:port]/<owner>/<name>[/<index>])",
            ));
        }
    };

    let c = match components.next() {
        Some(c) => c,
        None => {
            return Ok(service::Basis {
                host: host.to_owned(),
                owner: owner.to_owned(),
                name: name.to_owned(),
                ..Default::default()
            })
        }
    };

    if c == "change" {
        let change = match components.next().map(|i| i.parse::<u64>()) {
            Some(Ok(id)) => id,
            Some(Err(_)) => return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "failed to parse change id as number",
            )),
            None => return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid basis (must be of the form <host>[:port]/<owner>/<name>/change/<change_id>[/<index>])",
            )),
        };

        let index = match components.next().map(|i| i.parse::<u64>()) {
            Some(Ok(index)) => index,
            Some(Err(_)) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "failed to parse change index as number",
                ))
            }
            None => {
                return Ok(service::Basis {
                    host: host.to_owned(),
                    owner: owner.to_owned(),
                    name: name.to_owned(),
                    change,
                    ..Default::default()
                })
            }
        };

        return Ok(service::Basis {
            host: host.to_owned(),
            owner: owner.to_owned(),
            name: name.to_owned(),
            change,
            index,
        });
    }

    match c.parse::<u64>() {
        Ok(index) => {
            if components.next().is_some() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "unexpected trailing components after valid basis",
                ));
            }

            return Ok(service::Basis {
                host: host.to_owned(),
                owner: owner.to_owned(),
                name: name.to_owned(),
                index,
                ..Default::default()
            });
        }
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "failed to parse change index",
            ));
        }
    }
}

pub fn fmt_time(ts: u64) -> String {
    let now = timestamp_usec();
    let suffix = if now > ts { "ago" } else { "from now" };

    let seconds = if now > ts {
        (now - ts) / 1_000_000
    } else {
        (ts - now) / 1_000_000
    };
    if seconds < 10 {
        return String::from("just now");
    } else if seconds < 60 {
        return format!("{} seconds {}", seconds, suffix);
    } else if seconds < 120 {
        return format!("1 minute {}", suffix);
    }

    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{} minutes {}", minutes, suffix);
    } else if minutes < 120 {
        return format!("1 hour {}", suffix);
    }

    let hours = minutes / 60;
    if hours < 24 {
        return format!("{} hours {}", hours, suffix);
    } else if hours < 48 {
        return format!("1 day {}", suffix);
    }

    let days = hours / 24;
    if days < 7 {
        return format!("{} days {}", days, suffix);
    } else if days < 8 {
        return format!("1 week {}", suffix);
    }

    return format!("{} days {}", days, suffix);
}

pub fn decode_id(id: &str) -> std::io::Result<u64> {
    match u64::from_str_radix(id, 16) {
        Ok(x) => Ok(u64::MAX - x),
        Err(_) => Err(std::io::Error::from(std::io::ErrorKind::InvalidData)),
    }
}

pub fn encode_id(id: u64) -> String {
    format!("{:016x}", u64::MAX - id)
}

pub fn normalize_change_description(input: &str) -> String {
    let mut description = Vec::new();
    for line in input.split("\n") {
        let trimmed = line.trim();

        // Ignore comment lines
        if trimmed.starts_with("#") {
            continue;
        }

        // If the line is too full of spaces, strip them.
        if line.starts_with("   ") {
            description.push(trimmed);
            continue;
        }

        description.push(line);
    }

    // Remove starting and trailing newlines.
    let mut sliced_desc = description.as_slice();
    while let Some(&"") = sliced_desc.first() {
        sliced_desc = &sliced_desc[1..];
    }

    while let Some(&"") = sliced_desc.last() {
        sliced_desc = &sliced_desc[..sliced_desc.len() - 1];
    }

    sliced_desc.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_id() {
        assert_eq!(encode_id(12345678), "ffffffffff439eb1");
        assert!(encode_id(123) < encode_id(122));
        assert_eq!(decode_id(&encode_id(5938)).unwrap(), 5938);
    }

    #[test]
    fn test_diffs() {
        let left = "first\nsecond\nthird\n";
        let right = "first\nduo\nthird\n";
        let out = diff(left.as_bytes(), right.as_bytes());
        assert_eq!(
            out,
            vec![service::ByteDiff {
                start: 6,
                end: 13,
                kind: service::DiffKind::Modified,
                data: "duo\n".as_bytes().to_vec(),
                compression: service::CompressionKind::None,
            }]
        );
    }

    #[test]
    fn test_diffs_binary() {
        let left = &[1, 2, 3, 4];
        let right = &[1, 2, 11, 12];
        let out = diff(left, right);
        assert_eq!(
            out,
            vec![service::ByteDiff {
                start: 0,
                end: 4,
                kind: service::DiffKind::Modified,
                data: vec![1, 2, 11, 12],
                compression: service::CompressionKind::None,
            },]
        );
    }

    #[test]
    fn test_basis_parsing() {
        let b = parse_basis("src.colinmerkel.xyz:2020/colin/zork").unwrap();
        assert_eq!(
            b,
            service::Basis {
                host: "src.colinmerkel.xyz:2020".to_string(),
                owner: "colin".to_string(),
                name: "zork".to_string(),
                ..Default::default()
            }
        );

        let b = parse_basis("src.colinmerkel.xyz/colin/zork/5029").unwrap();
        assert_eq!(
            b,
            service::Basis {
                host: "src.colinmerkel.xyz".to_string(),
                owner: "colin".to_string(),
                name: "zork".to_string(),
                index: 5029,
                ..Default::default()
            }
        );

        let b = parse_basis("src.colinmerkel.xyz/colin/zork/change/5029/555").unwrap();
        assert_eq!(
            b,
            service::Basis {
                host: "src.colinmerkel.xyz".to_string(),
                owner: "colin".to_string(),
                name: "zork".to_string(),
                change: 5029,
                index: 555,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_hash() {
        let data = "asdf".as_bytes();
        assert_eq!(
            &fmt_sha(&hash_bytes(data)),
            "f0e4c2f76c58916ec258f246851bea091d14d4247a2fc3e18694461b1816e13b"
        );
    }

    #[test]
    fn test_fmt_sha() {
        assert_eq!(&fmt_sha(&[0, 0, 0, 0]), "00000000");
    }
}

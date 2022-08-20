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

pub fn diff(original: &[u8], modified: &[u8]) -> Vec<service::ByteDiff> {
    let (original_s, modified_s) =
        match (std::str::from_utf8(original), std::str::from_utf8(modified)) {
            (Ok(o), Ok(m)) => (o, m),
            _ => {
                // If the files are binary, don't compute a diff, just register a deletion and an
                // addition.
                return vec![
                    service::ByteDiff {
                        start: 0,
                        end: original.len() as u32,
                        kind: service::DiffKind::Removed,
                        data: vec![],
                    },
                    service::ByteDiff {
                        start: 0,
                        end: modified.len() as u32,
                        kind: service::DiffKind::Added,
                        data: modified.to_owned(),
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
                right_pos += right.len() as u32;
                while let Some(patience::DiffComponent::Insertion(right)) = diff_iter.peek() {
                    right_pos += right.len() as u32;
                    diff_iter.next();
                }
                out.push(service::ByteDiff {
                    start: left_pos,
                    end: left_pos,
                    kind: service::DiffKind::Added,
                    data: modified[start..right_pos as usize].to_owned(),
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
                });
            }
        }
    }
    out
}

pub fn fmt_sha(sha: &[u8]) -> String {
    let mut out = String::new();
    for &byte in sha {
        write!(&mut out, "{:x}", byte).unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diffs() {
        let left = "first\nsecond\nthird\n";
        let right = "first\nduo\nthird\n";
        let out = diff(left.as_bytes(), right.as_bytes());
        assert_eq!(
            out,
            vec![
                service::ByteDiff {
                    start: 6,
                    end: 6,
                    kind: service::DiffKind::Added,
                    data: "duo\n".as_bytes().to_vec(),
                },
                service::ByteDiff {
                    start: 6,
                    end: 13,
                    kind: service::DiffKind::Removed,
                    data: vec![],
                },
            ]
        );
    }

    #[test]
    fn test_diffs_binary() {
        let left = &[1, 2, 3, 4];
        let right = &[1, 2, 11, 12];
        let out = diff(left, right);
        assert_eq!(
            out,
            vec![
                service::ByteDiff {
                    start: 0,
                    end: 0,
                    kind: service::DiffKind::Added,
                    data: vec![1, 2, 11, 12],
                },
                service::ByteDiff {
                    start: 0,
                    end: 4,
                    kind: service::DiffKind::Removed,
                    data: vec![],
                },
            ]
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
}

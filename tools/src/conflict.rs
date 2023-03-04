use crate::render::{ConflictResolution, Snippet};
use service::DiffKind;

#[derive(Debug, PartialEq)]
pub enum ConflictResolutionOverride {
    Remote,
    Local,
    Merged(Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub enum MergeResult {
    Merged(service::FileDiff),
    Conflict(Vec<u8>, Vec<(service::ByteDiff, service::ByteDiff)>),
    IrreconcilableStateChange(service::DiffKind, service::DiffKind),
    Error(String),
}

pub fn merge(original: &[u8], left: &service::FileDiff, right: &service::FileDiff) -> MergeResult {
    if left == right {
        return MergeResult::Merged(left.clone());
    }

    match (left.kind, right.kind) {
        // OK, should try to merge
        (DiffKind::Modified, DiffKind::Modified) | (DiffKind::Added, DiffKind::Added) => {}

        // Both agree to remove the file
        (DiffKind::Removed, DiffKind::Removed) => return MergeResult::Merged(left.clone()),

        // Irreconcilable!
        _ => return MergeResult::IrreconcilableStateChange(left.kind, right.kind),
    }

    let mut left_s: Vec<(Snippet, Vec<&service::ByteDiff>)> = Vec::new();
    for diff in &left.differences {
        let snippet = Snippet::from_bytediff(&diff, original, 1);

        match left_s.last_mut() {
            Some((s, acc)) => {
                if s.merge(&snippet, original) {
                    acc.push(&diff);
                } else {
                    left_s.push((snippet, vec![&diff]));
                }
            }
            None => left_s.push((snippet, vec![&diff])),
        }
    }

    let mut right_s: Vec<(Snippet, Vec<&service::ByteDiff>)> = Vec::new();
    for diff in &right.differences {
        let snippet = Snippet::from_bytediff(&diff, original, 1);

        match right_s.last_mut() {
            Some((s, acc)) => {
                if s.merge(&snippet, original) {
                    acc.push(&diff);
                } else {
                    right_s.push((snippet, vec![&diff]));
                }
            }
            None => right_s.push((snippet, vec![&diff])),
        }
    }

    let mut left_iter = left_s.iter().peekable();
    let mut right_iter = right_s.iter().peekable();

    let mut non_conflicting_changes: Vec<service::ByteDiff> = Vec::new();
    let mut conflicting_changes: Vec<(service::ByteDiff, service::ByteDiff)> = Vec::new();
    let mut accepted_shift: isize = 0;
    loop {
        match (left_iter.peek(), right_iter.peek()) {
            (Some(l), Some(r)) => {
                match l.0.conflicts(&r.0) {
                    ConflictResolution::ThisThenThat => {
                        non_conflicting_changes
                            .extend(l.1.iter().map(|c| shift(*c, accepted_shift)));
                        left_iter.next();
                    }
                    ConflictResolution::ThatThenThis => {
                        non_conflicting_changes
                            .extend(r.1.iter().map(|c| shift(*c, accepted_shift)));
                        right_iter.next();
                    }
                    ConflictResolution::Overlapping => {
                        let mut left_conflict_zone = l.0.clone();
                        let mut right_conflict_zone = r.0.clone();

                        left_iter.next();
                        right_iter.next();

                        loop {
                            if left_conflict_zone > right_conflict_zone {
                                // The left conflict zone exceeds the right. Then check if the next
                                // right element conflicts with the left conflict zone.
                                if let Some(rn) = right_iter.peek() {
                                    if let ConflictResolution::Overlapping =
                                        left_conflict_zone.conflicts(&rn.0)
                                    {
                                        right_conflict_zone.force_merge(&rn.0, original);
                                        right_iter.next();
                                    } else {
                                        // The next snippet does not conflict, so we can stop expanding
                                        // it.
                                        break;
                                    }
                                } else {
                                    // There is no next snippet so we are done.
                                    break;
                                }
                            } else {
                                if let Some(ln) = left_iter.peek() {
                                    if let ConflictResolution::Overlapping =
                                        right_conflict_zone.conflicts(&ln.0)
                                    {
                                        left_conflict_zone.force_merge(&ln.0, original);
                                        left_iter.next();
                                    } else {
                                        // The next snippet does not conflict, so we can stop expanding
                                        // it.
                                        break;
                                    }
                                } else {
                                    // There is no next snippet so we are done.
                                    break;
                                }
                            }
                        }

                        let (non_conflicting, conflicting) = match deconflict_zones(
                            original,
                            &left_conflict_zone.into_bytediff(),
                            &right_conflict_zone.into_bytediff(),
                        ) {
                            Ok(r) => r,
                            Err(e) => return MergeResult::Error(format!("{e:?}")),
                        };

                        for resolution in non_conflicting {
                            let length = match crate::data_length(&resolution) {
                                Ok(l) => l,
                                Err(e) => return MergeResult::Error(format!("{e:?}")),
                            };

                            accepted_shift += length as isize
                                - (resolution.end as isize - resolution.start as isize);
                            non_conflicting_changes.push(resolution);
                        }

                        // Convert snippets into bytediffs
                        for (left, right) in conflicting {
                            conflicting_changes.push((
                                shift(&left, accepted_shift),
                                shift(&right, accepted_shift),
                            ));
                        }
                    }
                }
            }
            (Some(l), None) => {
                non_conflicting_changes.extend(l.1.iter().map(|c| shift(*c, accepted_shift)));
                left_iter.next();
            }
            (None, Some(r)) => {
                non_conflicting_changes.extend(r.1.iter().map(|c| shift(*c, accepted_shift)));
                right_iter.next();
            }
            (None, None) => break,
        }
    }

    let mut result: service::FileDiff = left.clone();
    result.differences = non_conflicting_changes
        .iter()
        .map(|b| (*b).clone())
        .collect();
    result.kind = service::DiffKind::Modified;

    if !conflicting_changes.is_empty() {
        let partially_merged = match crate::apply(result.as_view(), original) {
            Ok(r) => r,
            Err(e) => return MergeResult::Error(format!("{e:?}")),
        };
        return MergeResult::Conflict(partially_merged, conflicting_changes);
    }

    MergeResult::Merged(result)
}

fn shift(diff: &service::ByteDiff, shift: isize) -> service::ByteDiff {
    let mut adjusted = diff.clone();
    adjusted.start = ((adjusted.start as i32) + shift as i32) as u32;
    adjusted.end = ((adjusted.end as i32) + (shift as i32)) as u32;
    adjusted
}

// Takes diffs which overlap and tries to reduce the conflict by eliminating common changes
fn deconflict_zones(
    original: &[u8],
    left: &service::ByteDiff,
    right: &service::ByteDiff,
) -> std::io::Result<(
    Vec<service::ByteDiff>,
    Vec<(service::ByteDiff, service::ByteDiff)>,
)> {
    // Align the conflict zones to cover the same region of the original
    // document.
    let zone_start = std::cmp::min(left.start, right.start) as u32;
    let zone_end = std::cmp::max(left.end, right.end) as u32;

    let fragment = &original[zone_start as usize..zone_end as usize];
    let left_zone = {
        let mut adjusted = left.clone();
        adjusted.start -= zone_start;
        adjusted.end -= zone_start;
        crate::apply(
            service::FileDiff {
                differences: vec![adjusted],
                ..Default::default()
            }
            .as_view(),
            fragment,
        )?
    };
    let right_zone = {
        let mut adjusted = right.clone();
        adjusted.start -= zone_start;
        adjusted.end -= zone_start;
        crate::apply(
            service::FileDiff {
                differences: vec![adjusted],
                ..Default::default()
            }
            .as_view(),
            fragment,
        )?
    };

    let mut non_conflicting_changes = Vec::new();
    let mut conflicting_changes = Vec::new();

    let mut pos = 0;
    for diff in crate::diff(&left_zone, &right_zone) {
        if diff.start > pos {
            non_conflicting_changes.push(service::ByteDiff {
                start: zone_start,
                end: zone_start,
                kind: service::DiffKind::Added,
                data: left_zone[pos as usize..diff.start as usize].to_owned(),
                compression: service::CompressionKind::None,
            });
        }
        pos = diff.end;

        match diff.kind {
            service::DiffKind::Added => {
                let mut adjusted = diff.clone();
                adjusted.start = zone_start;
                adjusted.end = zone_start;
                non_conflicting_changes.push(adjusted);
            }
            service::DiffKind::Removed => {
                non_conflicting_changes.push(service::ByteDiff {
                    start: zone_start,
                    end: zone_start,
                    data: left_zone[diff.start as usize..diff.end as usize].to_owned(),
                    kind: service::DiffKind::Added,
                    compression: service::CompressionKind::None,
                });
            }
            service::DiffKind::Modified => {
                let mut adjusted = diff.clone();
                adjusted.start = zone_start;
                adjusted.end = zone_start;

                conflicting_changes.push((
                    adjusted,
                    service::ByteDiff {
                        start: zone_start,
                        end: zone_start,
                        data: left_zone[diff.start as usize..diff.end as usize].to_owned(),
                        kind: service::DiffKind::Added,
                        compression: service::CompressionKind::None,
                    },
                ));
            }
            _ => unreachable!(),
        }
    }
    if pos < left_zone.len() as u32 {
        non_conflicting_changes.push(service::ByteDiff {
            start: zone_start,
            end: zone_start,
            kind: service::DiffKind::Added,
            data: left_zone[pos as usize..].to_owned(),
            compression: service::CompressionKind::None,
        });
    }

    let mut includes_adjustment = false;
    if let Some(ch) = non_conflicting_changes.last_mut() {
        if ch.kind == service::DiffKind::Added {
            ch.kind = service::DiffKind::Modified;
        }
        ch.start = zone_start;
        ch.end = zone_end;
        includes_adjustment = true;
    }

    if !includes_adjustment {
        if let Some((ch1, ch2)) = conflicting_changes.last_mut() {
            if ch1.kind == service::DiffKind::Added {
                ch1.kind = service::DiffKind::Modified;
            }
            ch1.start = zone_start;
            ch1.end = zone_end;

            if ch2.kind == service::DiffKind::Added {
                ch2.kind = service::DiffKind::Modified;
            }
            ch2.start = zone_start;
            ch2.end = zone_end;
        }
    }

    Ok((non_conflicting_changes, conflicting_changes))
}

pub fn render_conflict(
    original: &[u8],
    conflicts: &[(service::ByteDiff, service::ByteDiff)],
    left_label: &str,
    right_label: &str,
) -> Vec<u8> {
    let mut pos: usize = 0;
    let mut out = Vec::new();
    for (left, right) in conflicts {
        assert_eq!(left.start, right.start);
        assert_eq!(left.end, right.end);

        if left.start as usize > pos {
            out.extend(&original[pos..left.start as usize]);
        }
        out.extend(format!("<<<<<<< {left_label}\n").as_bytes());
        out.extend(&crate::decompress(left.compression, &left.data).unwrap());
        out.extend("=======\n".as_bytes());
        out.extend(&crate::decompress(right.compression, &right.data).unwrap());
        out.extend(format!(">>>>>>> {right_label}\n").as_bytes());

        pos = left.end as usize;
    }

    if pos < original.len() {
        out.extend(&original[pos..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge() {
        let original = "a\nb\nc\nd\n";
        let modified = "a\nd\n";
        let result = try_merge(original, modified, modified);
        assert_merged(original, result, "a\nd\n");
    }

    fn try_merge(original: &str, modified_l: &str, modified_r: &str) -> MergeResult {
        let left = service::FileDiff {
            differences: crate::diff(original.as_bytes(), modified_l.as_bytes()),
            kind: service::DiffKind::Modified,
            ..Default::default()
        };
        let right = service::FileDiff {
            differences: crate::diff(original.as_bytes(), modified_r.as_bytes()),
            kind: service::DiffKind::Modified,
            ..Default::default()
        };

        merge(original.as_bytes(), &left, &right)
    }

    fn assert_merged(original: &str, result: MergeResult, expected: &str) {
        if let MergeResult::Merged(r) = result {
            let result = crate::apply(r.as_view(), original.as_bytes()).unwrap();
            println!("expected:\n\n{expected}\n\n");
            let result_str = std::str::from_utf8(result.as_slice()).unwrap();
            println!("got:\n\n{result_str}\n\n");
            assert_eq!(result_str, expected);
        } else {
            println!("expected successful merge, got: {result:?}");
            assert!(false);
        }
    }

    #[test]
    fn test_merge_complex() {
        let original = "a\nb\nc\nd\n";
        let modified_l = "a\nb\nbb\nc\nd\n";
        let modified_r = "a\nb\nc\ndd\n";
        let result = try_merge(original, modified_l, modified_r);

        assert_merged(original, result, "a\nb\nbb\nc\ndd\n")
    }

    #[test]
    fn test_merge_complex_2() {
        let original = "asdf\nfdsa\nqwerty\n";
        let modified_l = "new\nasdf\naaaa\nqwerty\n";
        let modified_r = "new\nasdf\nasdf\nqwerty\n";
        let result = try_merge(original, modified_l, modified_r);
        assert_merged(original, result, "new\nasdf\nasdf\naaaa\nqwerty\n");
    }

    #[test]
    fn test_merge_with_conflict() {
        let original = "asdf\nfdsa\nqwerty\n";
        let modified_l = "new\nasdf\naaaa\nqwerty\n";
        let modified_r = "new\nasdf\nbbbb\nqwerty\n";
        let result = try_merge(original, modified_l, modified_r);

        let (joined, conflicts) = match result {
            MergeResult::Conflict(joined, conflicts) => (joined, conflicts),
            _ => unreachable!(),
        };

        assert_eq!(
            std::str::from_utf8(&joined).unwrap(),
            "new\nasdf\nfdsa\nqwerty\n",
        );
        println!("left: {:#?}", conflicts[0].0);
        println!("right: {:#?}", conflicts[0].1);
        assert_eq!(conflicts[0].1.start, 9);
        assert_eq!(conflicts[0].1.end, 14);
    }
}

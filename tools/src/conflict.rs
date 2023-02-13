use crate::render::{ConflictResolution, Snippet};
use service::DiffKind;

#[derive(Debug, PartialEq)]
enum MergeResult {
    Merged(Vec<u8>),
    Removed,
    Conflict(Vec<u8>, Vec<(service::ByteDiff, service::ByteDiff)>),
    IrreconcilableStateChange(service::DiffKind, service::DiffKind),
    Error(String),
}

fn merge(original: &[u8], left: &service::FileDiff, right: &service::FileDiff) -> MergeResult {
    if left == right {
        return match crate::apply(left.as_view(), original) {
            Ok(r) => MergeResult::Merged(r),
            Err(e) => MergeResult::Error(format!("{e:?}")),
        };
    }

    match (left.kind, right.kind) {
        // OK, should try to merge
        (DiffKind::Modified, DiffKind::Modified) | (DiffKind::Added, DiffKind::Added) => {}

        // Both agree to remove the file
        (DiffKind::Removed, DiffKind::Removed) => return MergeResult::Removed,

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

    println!("left snippets: {left_s:#?}");

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

    println!("right snippets: {right_s:#?}");

    let mut left_iter = left_s.iter().peekable();
    let mut right_iter = right_s.iter().peekable();

    let mut non_conflicting_changes: Vec<&service::ByteDiff> = Vec::new();
    let mut conflicting_changes: Vec<(service::ByteDiff, service::ByteDiff)> = Vec::new();
    loop {
        match (left_iter.peek(), right_iter.peek()) {
            (Some(l), Some(r)) => {
                match l.0.conflicts(&r.0) {
                    ConflictResolution::ThisThenThat => {
                        non_conflicting_changes.extend(&l.1);
                        left_iter.next();
                    }
                    ConflictResolution::ThatThenThis => {
                        non_conflicting_changes.extend(&r.1);
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

                        // Convert snippets into bytediffs
                        conflicting_changes.push((
                            left_conflict_zone.into_bytediff(),
                            right_conflict_zone.into_bytediff(),
                        ));
                    }
                }
            }
            (Some(l), None) => {
                non_conflicting_changes.extend(&l.1);
                left_iter.next();
            }
            (None, Some(r)) => {
                non_conflicting_changes.extend(&r.1);
                right_iter.next();
            }
            (None, None) => break,
        }
    }

    let partially_merged = match crate::apply(
        service::FileDiff {
            differences: non_conflicting_changes
                .iter()
                .map(|b| (*b).clone())
                .collect(),
            kind: service::DiffKind::Modified,
            ..Default::default()
        }
        .as_view(),
        original,
    ) {
        Ok(r) => r,
        Err(e) => return MergeResult::Error(format!("{e:?}")),
    };

    if !conflicting_changes.is_empty() {
        return MergeResult::Conflict(partially_merged, conflicting_changes);
    }

    MergeResult::Merged(partially_merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge() {
        let original = "a\nb\nc\nd\n";
        let modified = "a\nd\n";
        let left = service::FileDiff {
            differences: crate::diff(original.as_bytes(), modified.as_bytes()),
            kind: service::DiffKind::Modified,
            ..Default::default()
        };
        let right = service::FileDiff {
            differences: crate::diff(original.as_bytes(), modified.as_bytes()),
            kind: service::DiffKind::Modified,
            ..Default::default()
        };

        let expected = MergeResult::Merged(Vec::from("a\nd\n"));
        assert_eq!(expected, merge(original.as_bytes(), &left, &right),);
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

    fn assert_merged(result: MergeResult, expected: &str) {
        if let MergeResult::Merged(r) = result {
            println!("expected:\n\n{expected}\n\n");
            let result_str = std::str::from_utf8(r.as_slice()).unwrap();
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

        assert_merged(result, "a\nb\nbb\nc\ndd\n")
    }
}

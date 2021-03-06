extern crate difference;
use difference::Difference;

#[derive(Clone, Debug, PartialEq)]
struct DiffChunk {
    start: usize,
    end: usize,
    has_contents: bool,
    contents: String,
}

#[derive(PartialEq)]
enum Position {
    Earlier,
    Later,
    Overlapping,
}

impl DiffChunk {
    fn new(start: usize) -> Self {
        DiffChunk {
            start: start,
            end: start,
            contents: String::new(),
            has_contents: false,
        }
    }

    fn relative_position(&self, other: &DiffChunk) -> Position {
        if other.end < self.start {
            Position::Earlier
        } else if other.start > self.end {
            Position::Later
        } else {
            Position::Overlapping
        }
    }
}

fn get_chunks(original: &str, modified: &str) -> Vec<DiffChunk> {
    let changeset = difference::Changeset::new(original, modified, "\n");
    let mut output = Vec::new();
    let mut line = 0;
    let mut in_progress = false;
    let mut chunk = DiffChunk::new(0);
    for diff in changeset.diffs {
        match diff {
            Difference::Same(s) => {
                if in_progress {
                    output.push(chunk.clone());
                    in_progress = false;
                }
                line += s.matches("\n").count() + 1;
            }
            Difference::Rem(s) => {
                if !in_progress {
                    in_progress = true;
                    chunk = DiffChunk::new(line);
                }
                line += s.matches("\n").count() + 1;
                chunk.end = line;
            }
            Difference::Add(s) => {
                if !in_progress {
                    in_progress = true;
                    chunk = DiffChunk::new(line);
                }
                chunk.has_contents = true;
                chunk.contents += &s;
            }
        }
    }
    if in_progress {
        output.push(chunk);
    }
    output
}

fn apply_chunks(original: &str, changes: &[DiffChunk]) -> String {
    // Apply the changes.
    let mut line = 0;
    let mut original_iter = original.lines();
    let mut output = Vec::new();
    for chunk in changes.iter().rev() {
        output.append(
            &mut (&mut original_iter)
                .take(chunk.start - line)
                .collect::<Vec<_>>(),
        );

        if chunk.has_contents {
            output.push(&chunk.contents);
        }

        (&mut original_iter)
            .take(chunk.end - chunk.start)
            .for_each(drop);

        line = chunk.end;
    }

    output.append(&mut (&mut original_iter).collect::<Vec<_>>());
    output.join("\n")
}

pub fn merge(original: &str, a: &str, b: &str) -> (String, bool) {
    let mut chunks_a = get_chunks(original, a);
    let mut chunks_b = get_chunks(original, b);

    let mut to_apply = Vec::new();
    let mut conflict = false;
    while chunks_a.len() > 0 && chunks_b.len() > 0 {
        if chunks_a.is_empty() || chunks_b.is_empty() {
            break;
        }

        let relative_position = {
            let a = chunks_a.last().unwrap();
            let b = chunks_b.last().unwrap();
            a.relative_position(b)
        };
        match relative_position {
            Position::Earlier => {
                to_apply.push(chunks_a.pop().unwrap());
            }
            Position::Later => {
                to_apply.push(chunks_b.pop().unwrap());
            }
            Position::Overlapping => {
                let mut a = vec![chunks_a.pop().unwrap()];
                let mut b = vec![chunks_b.pop().unwrap()];
                let mut current_chunk = DiffChunk::new(a[0].start);

                // Find the set of overlapping chunks that need to be merged manually.
                loop {
                    for c_i in a.iter().chain(b.iter()) {
                        if c_i.start < current_chunk.start {
                            current_chunk.start = c_i.start;
                        }
                        if c_i.end > current_chunk.end {
                            current_chunk.end = c_i.end
                        }
                    }

                    let mut take_a = false;
                    if let Some(c) = chunks_a.last() {
                        if c.relative_position(&current_chunk) == Position::Overlapping {
                            take_a = true;
                        }
                    }

                    let mut take_b = false;
                    if let Some(c) = chunks_b.last() {
                        if c.relative_position(&current_chunk) == Position::Overlapping {
                            take_b = true;
                        }
                    }

                    if take_a {
                        a.push(chunks_a.pop().unwrap());
                    }
                    if take_b {
                        b.push(chunks_b.pop().unwrap());
                    }

                    if !take_a && !take_b {
                        break;
                    }
                }

                // Remap the diffs so their line numbers reference the conflicting substring.
                a.iter_mut()
                    .map(|c| {
                        c.start -= current_chunk.start;
                        c.end -= current_chunk.start;
                    })
                    .for_each(drop);
                b.iter_mut()
                    .map(|c| {
                        c.start -= current_chunk.start;
                        c.end -= current_chunk.start;
                    })
                    .for_each(drop);

                let start_idx = match original
                    .match_indices("\n")
                    .take(current_chunk.start)
                    .last()
                {
                    Some((i, _)) => i + 1,
                    None => 0,
                };

                let end_idx = match original.match_indices("\n").take(current_chunk.end).last() {
                    Some((i, _)) => i + 1,
                    None => original.len(),
                };
                let conflicting_substr = &original[start_idx..end_idx];

                let version_a = apply_chunks(conflicting_substr, &a);
                let version_b = apply_chunks(conflicting_substr, &b);

                // It's possible that the two versions are actually sharing some modifications.
                // Any set of leading or trailing shared modifications should be allowed to go
                // outside the change markers.
                let mut changeset = difference::Changeset::new(&version_a, &version_b, "\n");
                let mut shared_changes_suffix = Vec::new();
                let mut pop = true;
                while pop {
                    pop = false;
                    {
                        if let Some(Difference::Same(s)) = changeset.diffs.last() {
                            pop = true;
                            shared_changes_suffix.push(s.to_owned());
                        }
                    }
                    if pop {
                        changeset.diffs.pop();
                    }
                }

                let mut shared_changes_prefix = Vec::new();
                let mut pop = true;
                while pop {
                    pop = false;
                    {
                        if let Some(Difference::Same(s)) = changeset.diffs.first() {
                            pop = true;
                            shared_changes_prefix.push(s.to_owned());
                        }
                    }
                    if pop {
                        changeset.diffs.remove(0);
                    }
                }

                // It's possible no real conflict exists - in which case, we can
                // just return the prefix and suffix combined together.
                if changeset.diffs.is_empty() {
                    shared_changes_prefix.append(&mut shared_changes_suffix);
                    current_chunk.contents = shared_changes_prefix.join("\n");
                    current_chunk.has_contents = true;
                    to_apply.push(current_chunk);
                    continue;
                }

                // Reconstruct versions A and B with shared changes factored out.
                let mut lines_a = Vec::new();
                let mut lines_b = Vec::new();
                for diff in changeset.diffs {
                    match diff {
                        Difference::Same(s) => {
                            lines_a.push(s.clone());
                            lines_b.push(s.clone());
                        }
                        Difference::Rem(s) => {
                            lines_a.push(s.clone());
                        }
                        Difference::Add(s) => {
                            lines_b.push(s.clone());
                        }
                    }
                }

                let version_a = lines_a.join("\n");
                let version_b = lines_b.join("\n");

                let conflict_markers = format!(
                    "<<<<<<< remote\n{}\n=======\n{}\n>>>>>>> local",
                    version_a, version_b
                );

                // Merge together the prefix, conflict markers, and suffix.
                shared_changes_prefix.push(conflict_markers);
                shared_changes_prefix.append(&mut shared_changes_suffix);

                conflict = true;
                current_chunk.contents = shared_changes_prefix.join("\n");
                current_chunk.has_contents = true;
                to_apply.push(current_chunk);
            }
        }
    }
    to_apply.extend(chunks_a.into_iter().rev());
    to_apply.extend(chunks_b.into_iter().rev());

    let output = apply_chunks(original, &to_apply) + "\n";
    (output, !conflict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_merge() {
        let (joined, ok) = merge("a brown cow", "a brown cow", "a cow");
        assert!(ok);
        assert_eq!(&joined, "a cow\n");
    }

    #[test]
    fn test_complex_safe_merge() {
        let (joined, ok) = merge(
            "a\nvery\nbrown\nold\ncow",
            "a\nvery\nred\nold\ncow",
            "a\nvery\nbrown\nold\ntomato\nwith vitamin c",
        );
        assert!(ok);
        assert_eq!(&joined, "a\nvery\nred\nold\ntomato\nwith vitamin c\n");
    }

    #[test]
    fn test_complex_safe_merge_2() {
        let (joined, ok) = merge(
            "start\ngets\ndeleted\nmiddle\npart\nstays\nend\ndeleted\n",
            "middle\npart\nstays",
            "start\ngets\ndeleted\nmiddle\nbit\nstays\nend\ndeleted\n",
        );
        assert!(ok);
        assert_eq!(&joined, "middle\nbit\nstays\n");
    }

    #[test]
    fn test_complex_safe_merge_3() {
        let (joined, ok) = merge(
            "start\ngets\ndeleted\nmiddle\npart\nstays\nend\ndeleted\n",
            "start\ngets\ndeleted\nmiddle\npart\nstays\nend\ndeleted\n",
            "middle\npart\nstays",
        );
        assert!(ok);
        assert_eq!(&joined, "middle\npart\nstays\n");
    }

    #[test]
    fn test_complex_safe_merge_4() {
        let (joined, ok) = merge(
            "start\ngets\ndeleted\nmiddle\npart\nstays\nend\ndeleted\n",
            "start\ngets\ndeleted\nmiddle\npart\nstays\nend\ndeleted\n",
            "dtart\ngets\naeleted\nmiddle\ndart\nstays\nand\ndeleted\n",
        );
        assert!(ok);
        assert_eq!(
            &joined,
            "dtart\ngets\naeleted\nmiddle\ndart\nstays\nand\ndeleted\n"
        );
    }

    #[test]
    fn test_merge_conflicts() {
        let (joined, ok) = merge(
            "starting line\nmodified line\nending line\n",
            "starting line\nmodified slime\nending line\n",
            "starting line\nmodified climb\nending line\n",
        );
        assert!(!ok);
        assert_eq!(&joined, "starting line\n<<<<<<< remote\nmodified slime\n=======\nmodified climb\n>>>>>>> local\nending line\n",);
    }

    #[test]
    fn test_complex_merge_conflicts() {
        let (joined, ok) = merge(
            "L1\nL2\nL3\nL4\nL5\nL6\nL7\nL8",
            "L8",
            "L1\nM2\nL3\nM4\nL5\nL6\nL7\nL8",
        );
        assert!(!ok);
        assert_eq!(
            &joined,
            "<<<<<<< remote\n\n=======\nL1\nM2\nL3\nM4\nL5\nL6\nL7\n>>>>>>> local\nL8\n"
        );
    }

    #[test]
    fn test_complex_merge_conflicts_2() {
        let (joined, ok) = merge(
            "L1\nL2\nL3\nL4\nL5\nL6\nL7\nL8",
            "L1\nL4\nL7\nL8",
            "L1\nL2\nL3\nL6\nL7\nL8",
        );
        assert!(!ok);
        assert_eq!(
            &joined,
            "L1\n<<<<<<< remote\nL4\n=======\nL2\nL3\nL6\n>>>>>>> local\nL7\nL8\n"
        );
    }

    #[test]
    fn test_non_conflicting_conflict() {
        let (joined, ok) = merge(
            "original string",
            "conflicting identical change",
            "conflicting identical change",
        );
        assert!(ok);
        assert_eq!(&joined, "conflicting identical change\n");
    }

    #[test]
    fn test_partially_non_conflicting_conflict() {
        let (joined, ok) = merge("L1\nL2\nL3\nL4", "M1\nM2\nM3\nL4", "M1\nK2\nM3\nL4");
        assert!(!ok);
        assert_eq!(
            &joined,
            "M1\n<<<<<<< remote\nM2\n=======\nK2\n>>>>>>> local\nM3\nL4\n"
        );
    }

    #[test]
    fn test_chunks() {
        let original = "the same line\na different line\na third line";
        let modified = "the same line\nI took a DNA test\nturns out\na third line";

        let chunks = get_chunks(original, modified);

        let mut expected = DiffChunk::new(1);
        expected.contents = String::from("I took a DNA test\nturns out");
        expected.has_contents = true;
        expected.end = 2;

        assert_eq!(chunks, vec![expected]);
    }

    #[test]
    fn test_chunks_add() {
        let original = "hello world\nanother line\n";
        let modified = "hello world\na whole new line\nanother line\n";

        let chunks = get_chunks(original, modified);

        let mut expected = DiffChunk::new(1);
        expected.contents = String::from("a whole new line");
        expected.has_contents = true;
        expected.end = 1;

        assert_eq!(chunks, vec![expected]);
    }

    #[test]
    fn test_weird_merge() {
        let original = "read us all";
        let a = "heat exposes it\n\nread us all\n\nan obvious one\n";
        let b = "conflict\nread us all\n\nyou\n";

        let (joined, ok) = merge(original, a, b);
        assert!(!ok);
        // TODO(b/20): fix this test. I think the result is wrong and probably indicates a bug with
        // the merging code.
        assert_eq!(&joined, "<<<<<<< remote\nheat exposes it\n\n=======\nconflict\n>>>>>>> local\nread us all\nread us all\n\n<<<<<<< remote\nan obvious one\n=======\nyou\n>>>>>>> local\n\nread us all\n");
    }
}

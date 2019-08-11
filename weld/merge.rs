extern crate difference;
use difference::Difference;

#[derive(Clone, Debug, PartialEq)]
struct DiffChunk {
    start: usize,
    end: usize,
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
                println!("same: {}", s);
                if in_progress {
                    output.push(chunk.clone());
                    in_progress = false;
                }
                line += s.matches("\n").count() + 1;
            }
            Difference::Rem(s) => {
                println!("rem: {}", s);
                if !in_progress {
                    in_progress = true;
                    chunk = DiffChunk::new(line);
                }
                line += s.matches("\n").count() + 1;
                chunk.end = line;
            }
            Difference::Add(s) => {
                println!("rem: {}", s);
                if !in_progress {
                    in_progress = true;
                    chunk = DiffChunk::new(line);
                }
                chunk.contents += &s;
            }
        }
    }
    if in_progress {
        output.push(chunk);
    }
    output
}

pub fn merge(original: &str, a: &str, b: &str) -> (String, bool) {
    let mut chunks_a = get_chunks(original, a);
    let mut chunks_b = get_chunks(original, b);

    println!("chunks_a: {:?}", chunks_a);
    println!("chunks_b: {:?}", chunks_b);

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
                println!("add: {:?}", chunks_a.last().unwrap());
                to_apply.push(chunks_a.pop().unwrap());
            }
            Position::Later => {
                println!("add: {:?}", chunks_b.last().unwrap());
                to_apply.push(chunks_b.pop().unwrap());
            }
            Position::Overlapping => {
                conflict = true;
                let mut a = vec![chunks_a.pop().unwrap()];
                let mut b = vec![chunks_b.pop().unwrap()];
                let mut current_chunk = DiffChunk::new(a[0].start);

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
                        b.push(chunks_a.pop().unwrap());
                    }

                    if !take_a && !take_b {
                        break;
                    }
                }

                current_chunk.contents = String::from("merge conflict");
                to_apply.push(current_chunk);
            }
        }
    }
    to_apply.extend(chunks_a.into_iter().rev());
    to_apply.extend(chunks_b.into_iter().rev());

    println!("to_apply: {:?}", to_apply);

    // Apply the changes.
    let mut line = 0;
    let mut original_iter = original.lines();
    let mut output = Vec::new();
    for chunk in to_apply.iter().rev() {
        output.append(
            &mut (&mut original_iter)
                .take(chunk.start - line)
                .collect::<Vec<_>>(),
        );

        if !chunk.contents.is_empty() {
            output.push(&chunk.contents);
        }

        (&mut original_iter)
            .take(chunk.end - chunk.start)
            .for_each(drop);

        line = chunk.end;
    }

    let output = output.join("\n");

    println!("{}", output);

    (output, !conflict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_merge() {
        let (joined, ok) = merge("a brown cow", "a brown cow", "a cow");
        assert!(ok);
        assert_eq!(&joined, "a cow");
    }

    #[test]
    fn test_complex_safe_merge() {
        let (joined, ok) = merge(
            "a\nvery\nbrown\nold\ncow",
            "a\nvery\nred\nold\ncow",
            "a\nvery\nbrown\nold\ntomato\nwith vitamin c",
        );
        assert!(ok);
        assert_eq!(&joined, "a\nvery\nred\nold\ntomato\nwith vitamin c");
    }

    #[test]
    fn test_complex_safe_merge_2() {
        let (joined, ok) = merge(
            "start\ngets\ndeleted\nmiddle\npart\nstays\nend\ndeleted\n",
            "middle\npart\nstays",
            "start\ngets\ndeleted\nmiddle\nbit\nstays\nend\ndeleted\n",
        );
        assert!(ok);
        assert_eq!(&joined, "middle\nbit\nstays");
    }

    #[test]
    fn test_chunks() {
        let original = "the same line\na different line\na third line";
        let modified = "the same line\nI took a DNA test\nturns out\na third line";

        let chunks = get_chunks(original, modified);

        let mut expected = DiffChunk::new(1);
        expected.contents = String::from("I took a DNA test\nturns out");
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
        expected.end = 1;

        assert_eq!(chunks, vec![expected]);
    }
}

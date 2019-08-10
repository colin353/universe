extern crate difference;
use difference::Difference;

#[derive(Clone, Debug, PartialEq)]
struct DiffChunk {
    start: usize,
    end: usize,
    contents: String,
}

impl DiffChunk {
    fn new(start: usize) -> Self {
        DiffChunk {
            start: start,
            end: start,
            contents: String::new(),
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
    (String::from(a), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_merge() {
        let (joined, ok) = merge("a brown cow", "a cow", "a cow");
        assert!(ok);
        assert_eq!(&joined, "a cow");
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
        expected.end = 2;

        assert_eq!(chunks, vec![expected]);
    }
}

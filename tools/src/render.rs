use crate::patience;
use std::fmt::Write;

pub fn print_diff(files: &[service::FileDiff]) {
    // Print added files
    let mut first = true;
    for file in files.iter().filter(|f| f.kind == service::DiffKind::Added) {
        if first {
            first = false;
            println!("Added: ");
        }
        println!("    {} [+{}]", file.path, 325);
    }

    // Print modified files
    let mut first = true;
    for file in files
        .iter()
        .filter(|f| f.kind == service::DiffKind::Modified)
    {
        if first {
            first = false;
            println!("Modified: ");
        }
        println!("    {} [+{} -{}]", file.path, 2, 6);
    }

    // Print deleted files
    let mut first = true;
    for file in files
        .iter()
        .filter(|f| f.kind == service::DiffKind::Removed)
    {
        if first {
            first = false;
            println!("Deleted: ");
        }
        println!("    {} [-{}]", file.path, 64);
    }
}

#[derive(Debug)]
struct Snippet {
    old_start_pos: usize,
    old_end_pos: usize,
    modified_start_pos: usize,
    modified_end_pos: usize,
    new_data: Vec<u8>,
}

impl Snippet {
    fn from_bytediff(diff: &service::ByteDiff, prev_data: &[u8]) -> Self {
        let mut old_start_pos = diff.start as usize;
        let mut line_idx = 0;
        for byte in prev_data[0..diff.start as usize].iter().rev() {
            if *byte == 0x0a {
                line_idx += 1;

                if line_idx == 3 {
                    break;
                }
            }
            old_start_pos -= 1;
        }

        let mut old_end_pos = diff.end as usize;
        for byte in prev_data[diff.end as usize..].iter() {
            if *byte == 0x0a {
                line_idx += 1;

                if line_idx == 3 {
                    break;
                }
            }
            old_end_pos += 1;
        }

        let mut new_data = prev_data[old_start_pos..diff.start as usize].to_owned();
        let modified_start_pos = new_data.len();
        new_data.extend_from_slice(diff.data.as_slice());
        let modified_end_pos = new_data.len();
        new_data.extend_from_slice(&prev_data[diff.end as usize..old_end_pos]);

        return Snippet {
            old_start_pos: old_start_pos as usize,
            old_end_pos: old_end_pos as usize,
            modified_start_pos,
            modified_end_pos,
            new_data,
        };
    }

    pub fn merge(&mut self, next_snippet: &Snippet, prev_data: &[u8]) -> bool {
        // Merge if the snippets overlap
        if next_snippet.old_start_pos > self.old_end_pos {
            return false;
        }

        self.old_end_pos = next_snippet.old_end_pos;

        // --prefix--[modified]--suffix
        //              ---prefix--[modified]--suffix---
        //                    ^
        //                    |
        //                    modified_end_pos

        // Only retain up to the modified end position
        self.new_data.truncate(self.modified_end_pos);

        // Re-insert the unmodified filler between the snippets
        self.new_data.extend(
            &prev_data[self.old_start_pos + self.modified_start_pos
                ..next_snippet.old_start_pos + next_snippet.modified_start_pos],
        );

        self.modified_end_pos =
            next_snippet.old_start_pos + next_snippet.modified_end_pos - self.old_start_pos;

        // Append the remainder of the other snippet
        self.new_data
            .extend(&next_snippet.new_data[next_snippet.modified_start_pos..]);

        true
    }
}

pub fn print_patch(
    from: &str,
    subject: &str,
    files: &[(&service::FileDiff, Option<&service::Blob>)],
) -> String {
    let mut out = String::new();
    writeln!(&mut out, "From: {}", from).unwrap();
    writeln!(&mut out, "Subject: [PATCH 1/1] {}\n", subject).unwrap();

    for (fd, prev) in files {
        match fd.kind {
            service::DiffKind::Modified => {
                let prev = match prev {
                    Some(p) => p,
                    None => continue,
                };

                writeln!(&mut out, "--- a/{}", fd.path).unwrap();
                writeln!(&mut out, "+++ b/{}", fd.path).unwrap();

                let mut snippets: Vec<Snippet> = Vec::new();
                let mut current_snippet: Option<Snippet> = None;

                for bd in &fd.differences {
                    let s = Snippet::from_bytediff(bd, &prev.data);
                    if let Some(ps) = &mut current_snippet {
                        if !ps.merge(&s, &prev.data) {
                            snippets.push(std::mem::replace(ps, s));
                        }
                    } else {
                        current_snippet = Some(s);
                    }
                }

                if let Some(s) = current_snippet {
                    snippets.push(s);
                }

                let offset = 0;
                for snippet in snippets {
                    let old_start_line_number = prev.data[0..snippet.old_start_pos]
                        .iter()
                        .filter(|b| **b == 0x0a)
                        .count()
                        + 1;
                    let old_line_span = prev.data[snippet.old_start_pos..snippet.old_end_pos]
                        .iter()
                        .filter(|b| **b == 0x0a)
                        .count();
                    let new_line_span = snippet.new_data.iter().filter(|b| **b == 0x0a).count();
                    let new_start_line_number = old_start_line_number + offset;

                    writeln!(
                        &mut out,
                        "@@ -{},{} +{},{} @@",
                        old_start_line_number, old_line_span, new_start_line_number, new_line_span
                    )
                    .unwrap();

                    let old = &prev.data[snippet.old_start_pos..snippet.old_end_pos];
                    let old_s = std::str::from_utf8(old).unwrap();
                    let mut old_lines = Vec::new();
                    let mut pos = 0;
                    for (idx, _) in old_s.match_indices('\n') {
                        old_lines.push(&old[pos..idx + 1]);
                        pos = idx + 1;
                    }
                    if !&old[pos..].is_empty() {
                        old_lines.push(&old[pos..]);
                    }

                    let new = &snippet.new_data;
                    let new_s = std::str::from_utf8(&snippet.new_data).unwrap();
                    let mut new_lines = Vec::new();
                    let mut pos = 0;
                    for (idx, _) in new_s.match_indices('\n') {
                        new_lines.push(&new[pos..idx + 1]);
                        pos = idx + 1;
                    }
                    if !&new[pos..].is_empty() {
                        new_lines.push(&old[pos..]);
                    }

                    for diff in patience::patience_diff(&old_lines, &new_lines) {
                        match diff {
                            patience::DiffComponent::Unchanged(left, _) => {
                                write!(&mut out, " {}", std::str::from_utf8(left).unwrap())
                                    .unwrap();
                            }
                            patience::DiffComponent::Insertion(right) => {
                                write!(&mut out, "+{}", std::str::from_utf8(right).unwrap())
                                    .unwrap();
                            }
                            patience::DiffComponent::Deletion(left) => {
                                write!(&mut out, "-{}", std::str::from_utf8(left).unwrap())
                                    .unwrap();
                            }
                        }
                    }
                }
            }
            service::DiffKind::Added => {
                writeln!(&mut out, "--- /dev/null").unwrap();
                writeln!(&mut out, "+++ b/{}", fd.path).unwrap();

                let mut buf = Vec::new();
                let mut content: &[u8] = &[];
                for bd in &fd.differences {
                    if bd.kind != service::DiffKind::Added {
                        continue;
                    }

                    if bd.compression == service::CompressionKind::None {
                        content = bd.data.as_slice();
                    } else {
                        buf = match crate::decompress(bd.compression, &bd.data) {
                            Ok(b) => b,
                            Err(_) => continue,
                        };
                        content = buf.as_slice();
                    }
                    break;
                }

                let content_str = match std::str::from_utf8(&content) {
                    Ok(c) => c,
                    // TODO: handle binary content
                    Err(_) => continue,
                };
                let line_count = content_str.lines().count();
                writeln!(&mut out, "@@ -0,0 +1,{} @@", line_count).unwrap();
                for line in content_str.lines() {
                    writeln!(&mut out, "+{}", line).unwrap();
                }
            }
            _ => {}
        }
    }

    out
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::diff;

    #[test]
    fn test_patch() {
        let original = "fn main() {
    println!(\"hello, world\");
}
";

        let new = "// comment
fn main() {
    println!(\"hello world\");
}
";
        let bytediffs = diff(original.as_bytes(), new.as_bytes());
        let filediff = service::FileDiff {
            path: "code.rs".to_string(),
            kind: service::DiffKind::Modified,
            is_dir: false,
            differences: bytediffs,
        };

        let blob = service::Blob {
            sha: vec![1, 2, 3, 4, 5],
            data: original.as_bytes().to_owned(),
        };

        let patch_ingredients = vec![(&filediff, Some(&blob))];
        let patch = print_patch("Colin", "asdf", patch_ingredients.as_slice());

        let expected = "From: Colin
Subject: [PATCH 1/1] asdf

--- a/code.rs
+++ b/code.rs
@@ -1,3 +1,4 @@
+// comment
 fn main() {
+    println!(\"hello world\");
-    println!(\"hello, world\");
 }
";

        println!(
            "expected patch:\n{}\n\nactual patch:\n{}\n",
            expected, patch
        );
        assert_eq!(patch, expected);
    }

    #[test]
    fn test_complex_patch() {
        let original = "
first line
second line
third line
fourth line
fifth line
sixth line
seventh line
eighth line
ninth line
many more lines
will come after
this one
eventually
";

        let new = "
first line
second line
third line
fourth line
fifth line
sixth line
seventh line
eighth line
ninth line
many more lines
will come after
this one
eventually...
but now I did add an extra line
";

        let bytediffs = diff(original.as_bytes(), new.as_bytes());
        let filediff = service::FileDiff {
            path: "code.rs".to_string(),
            kind: service::DiffKind::Modified,
            is_dir: false,
            differences: bytediffs,
        };

        let blob = service::Blob {
            sha: vec![1, 2, 3, 4, 5],
            data: original.as_bytes().to_owned(),
        };

        let patch_ingredients = vec![(&filediff, Some(&blob))];
        let patch = print_patch("Colin", "asdf", patch_ingredients.as_slice());

        let expected = "From: Colin
Subject: [PATCH 1/1] asdf

--- a/code.rs
+++ b/code.rs
@@ -12,3 +12,4 @@
 will come after
 this one
+eventually...
+but now I did add an extra line
-eventually
";

        println!(
            "expected patch:\n{}\n\nactual patch:\n{}\n",
            expected, patch
        );
        assert_eq!(patch, expected);
    }
}

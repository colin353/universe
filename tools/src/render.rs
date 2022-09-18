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

pub fn print_patch(
    from: &str,
    subject: &str,
    files: &[(&service::FileDiff, Option<&service::Blob>)],
) -> String {
    let mut out = String::new();
    writeln!(&mut out, "From: {}", from);
    writeln!(&mut out, "Subject: [PATCH 1/1] {}\n", subject);

    for (fd, prev) in files {
        match fd.kind {
            service::DiffKind::Modified => {
                let prev = match prev {
                    Some(p) => p,
                    None => continue,
                };

                writeln!(&mut out, "--- a/{}", fd.path);
                writeln!(&mut out, "+++ b/{}", fd.path);

                let mut offset = 0;

                for bd in &fd.differences {
                    match bd.kind {
                        service::DiffKind::Added => {
                            let old_start_line = prev.data[0..bd.start as usize]
                                .iter()
                                .filter(|b| **b == 0x0a)
                                .count()
                                + 1;
                            let old_length = prev.data[bd.start as usize..bd.end as usize]
                                .iter()
                                .filter(|b| **b == 0x0a)
                                .count()
                                + 1;
                            let new_start_line = old_start_line + offset;
                            let new_length = bd.data.iter().filter(|b| **b == 0x0a).count() + 1;
                            writeln!(
                                &mut out,
                                "@@ -{},{} +{},{}",
                                old_start_line, old_length, new_start_line, new_length
                            );
                        }
                        _ => continue,
                    }
                }
            }
            _ => continue,
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
    println!(\"hello world!\");
}
";
        let bytediffs = diff(original.as_bytes(), new.as_bytes());
        let filediff = service::FileDiff {
            path: "code.rs".to_string(),
            kind: service::DiffKind::Modified,
            is_dir: false,
            differences: bytediffs,
        };

        println!("{filediff:#?}");

        let blob = service::Blob {
            sha: vec![1, 2, 3, 4, 5],
            data: new.as_bytes().to_owned(),
        };

        let patch_ingredients = vec![(&filediff, Some(&blob))];
        let patch = print_patch("Colin", "asdf", patch_ingredients.as_slice());

        let expected = "--- a/code.rs
+++ b/code.rs
@@ -1,3 +1,4 @@
+// hello
 fn main() {
-    println!(\"hello, world\");
+    println!(\"hello world!\");
 }
";

        println!(
            "expected patch:\n{}\n\nactual patch:\n{}\n",
            expected, patch
        );
        assert_eq!(patch, expected);
    }
}

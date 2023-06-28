fn render_change_description(input: &str) -> String {
    let mut output = String::new();
    let mut chunk = Vec::new();
    let mut lines_iter = input.lines().map(|line| line.trim()).peekable();
    while let Some(_) = lines_iter.peek() {
        loop {
            let line = lines_iter.peek();
            if line.is_some() && line.unwrap().starts_with("- ") {
                chunk.push(&line.unwrap()[2..]);
                lines_iter.next();
            } else {
                if let Some(&"") = line {
                    lines_iter.next();
                }

                if !chunk.is_empty() {
                    output += "<ul>";
                    for c in &chunk {
                        output += "<li>";
                        output += &format!("{}", escape::Escape(c));
                        output += "</li>\n";
                    }
                    output += "</ul>\n";
                    chunk.clear();
                }
                break;
            }
        }

        loop {
            let line = lines_iter.peek();
            if line.is_none() || line.unwrap().starts_with("- ") || line.unwrap().is_empty() {
                if let Some(&"") = line {
                    lines_iter.next();
                }

                if !chunk.is_empty() {
                    output += "<p>";
                    for c in &chunk {
                        output += &format!("{}", escape::Escape(c));
                        output += "\n";
                    }
                    output += "</p>\n";
                    chunk.clear();
                }
                break;
            } else {
                chunk.push(line.unwrap());
                lines_iter.next();
            }
        }
    }
    output
}

pub fn change(c: &service::Change) -> tmpl::ContentsMap {
    tmpl::content!(
        "id" => format!("{}", c.id),
        "repo_owner" => c.repo_owner.clone(),
        "repo_name" => c.repo_name.clone(),
        "submitted_id" => format!("{}", c.submitted_id),
        // TODO: parse the summary out of the description
        "summary" => core::summarize_description(&c.description).to_owned(),
        "description" => render_change_description(&c.description),
        "author" => c.owner.clone(),
        "status" => format!("{:?}", c.status).to_uppercase();
        "tasks" => vec![]
    )
}

pub fn file_diff(s: &service::FileDiff) -> tmpl::ContentsMap {
    tmpl::content!(
        "path" => s.path.to_string(),
        "language" => format!("{:?}", language_specific::get_filetype(&s.path)).to_lowercase(),
        "is_dir" => s.is_dir,
        "kind" => format!("{:?}", s.kind)
    )
}

pub fn file_history(
    s: &service::FileDiff,
    original: Vec<u8>,
    modified: Vec<u8>,
) -> tmpl::ContentsMap {
    let mut original_is_binary = false;
    let original_s = match String::from_utf8(original) {
        Ok(s) => s,
        Err(_) => {
            original_is_binary = true;
            String::new()
        }
    };

    let mut modified_is_binary = false;
    let modified_s = match String::from_utf8(modified) {
        Ok(s) => s,
        Err(_) => {
            modified_is_binary = true;
            String::new()
        }
    };

    tmpl::content!(
        "path" => s.path.to_string(),
        "language" => format!("{:?}", language_specific::get_filetype(&s.path)).to_lowercase(),
        "original_is_binary" => original_is_binary,
        "original" => base64::encode(&original_s),
        "modified" => base64::encode(&modified_s),
        "modified_is_binary" => modified_is_binary,
        "is_dir" => s.is_dir,
        "kind" => format!("{:?}", s.kind)
    )
}

pub fn snapshot(s: &service::Snapshot) -> tmpl::ContentsMap {
    let basis = core::fmt_basis(s.basis.as_view());
    let mut short_basis = basis.split("/").skip(3).collect::<Vec<_>>().join("/");
    if short_basis.is_empty() {
        short_basis = "0".to_string();
    }

    tmpl::content!(
        "timestamp" => format!("{:?}", s.timestamp),
        "basis" => core::fmt_basis(s.basis.as_view()),
        "short_basis" => short_basis
        ;
        "files" => s.files.iter().map(|f| file_diff(f)).collect()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_description_rendering() {
        let input = "My content asdf

 - One
 - Two
 - Three

 asdf
";

        let expected = "<p>My content asdf
</p>
<ul><li>One</li>
<li>Two</li>
<li>Three</li>
</ul>
<p>asdf
</p>
";

        assert_eq!(render_change_description(input), expected);
    }
}

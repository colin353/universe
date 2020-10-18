fn starts_with_digits(line: &str) -> Option<usize> {
    let mut chars = line.chars();
    let mut offset = 0;
    for ch in chars {
        if ch.is_numeric() {
            offset += 1;
            continue;
        }

        if ch == '.' && offset != 0 {
            return Some(offset + 1);
        }

        return None;
    }

    None
}

pub fn to_html(input: &str) -> String {
    let mut output = String::new();
    let mut line_iter = input.lines().peekable();
    while let Some(line) = line_iter.next() {
        let line = line.trim();

        if line.starts_with("#") {
            output += &extract_header(line);
        } else if line == "```" {
            output += "<pre class='md-code'>";
            while let Some(l) = line_iter.next() {
                if l.trim() == "```" {
                    break;
                }

                output += l;
                output += "\n";
            }
            output += "</pre>";
        } else if line.is_empty() {
            while let Some(l) = line_iter.peek() {
                if l.trim().is_empty() {
                    line_iter.next();
                } else {
                    break;
                }
            }
        } else if let Some(offset) = starts_with_digits(line.trim()) {
            output += "<ol><li>";
            output += &fragment_to_html(&line.trim()[offset..]);

            while let Some(line) = line_iter.next() {
                if line.trim().is_empty() {
                    break;
                }

                if let Some(offset) = starts_with_digits(line.trim()) {
                    output += "</li>\n<li>";
                    output += &fragment_to_html(&line.trim()[offset..]);
                } else {
                    output += "\n";
                    output += &fragment_to_html(&line.trim());
                }
            }

            output += "</li></ol>\n";
        } else if line.starts_with("- ") {
            output += "<ul><li>";
            output += &fragment_to_html(&line[2..]);

            while let Some(line) = line_iter.next() {
                let line = line.trim();
                if line.is_empty() {
                    break;
                }

                if line.starts_with("- ") {
                    output += "</li>\n<li>";
                    output += &fragment_to_html(&line[2..]);
                } else {
                    output += "\n";
                    output += &fragment_to_html(line);
                }
            }

            output += "</li></ul>\n";
        } else {
            output += "<p>";
            output += &fragment_to_html(line);

            while let Some(line) = line_iter.next() {
                if line.trim().is_empty() {
                    break;
                }
                output += "\n";
                output += &fragment_to_html(line);
            }

            output += "</p>\n";
        }
    }

    output
}

pub fn fragment_to_html(text: &str) -> String {
    let mut out = String::new();
    let mut open = false;
    for chunk in text.split("`") {
        if !open {
            open = true;
            out += chunk;
        } else {
            open = false;
            out += &format!("<span class='md-code'>{}</span>", chunk);
        }
    }
    out
}

pub fn extract_header(line: &str) -> String {
    let mut prefix_length = 1;
    if line.starts_with("####") {
        prefix_length = 4;
    } else if line.starts_with("###") {
        prefix_length = 3;
    } else if line.starts_with("##") {
        prefix_length = 2;
    }

    format!(
        "<h{}>{}</h{}>\n",
        prefix_length,
        &fragment_to_html(line[prefix_length..].trim()),
        prefix_length
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragment_to_html() {
        assert_eq!(
            &fragment_to_html("my `special` code"),
            "my <span class='md-code'>special</span> code"
        );
    }

    #[test]
    fn test_code_fragment() {
        assert_eq!(
            &to_html("my `special` code"),
            "<p>my <span class='md-code'>special</span> code</p>\n"
        );
        assert_eq!(
            &to_html("# Big `header`"),
            "<h1>Big <span class='md-code'>header</span></h1>\n"
        );
        assert_eq!(
            &to_html(" - My `super` list\n- another elemenet\n continuation"),
            "<ul><li>My <span class='md-code'>super</span> list</li>\n<li>another elemenet\ncontinuation</li></ul>\n"
        );
    }

    #[test]
    fn test_code_block() {
        assert_eq!(
            to_html("```\nmy crazy code block\n\tanother line\n```"),
            "<pre class='md-code'>my crazy code block\n\tanother line\n</pre>"
        );
    }

    #[test]
    fn test_convert() {
        assert_eq!(to_html("# Big header"), "<h1>Big header</h1>\n");
        assert_eq!(to_html("##Secondary header"), "<h2>Secondary header</h2>\n");
        assert_eq!(
            to_html("Regular text\nwith more text\n\n## Subheader"),
            "<p>Regular text\nwith more text</p>\n<h2>Subheader</h2>\n"
        );
    }

    #[test]
    fn test_lists() {
        assert_eq!(
            to_html(" - My super list\n- another elemenet\n continuation"),
            "<ul><li>My super list</li>\n<li>another elemenet\ncontinuation</li></ul>\n"
        );
    }

    #[test]
    fn test_embedded_html() {
        assert_eq!(
            to_html("<iframe src=test />"),
            "<p><iframe src=test /></p>\n"
        );
    }
}

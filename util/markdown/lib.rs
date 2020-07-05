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
        if line.starts_with("#") {
            output += &extract_header(line);
        } else if line.trim().is_empty() {
            while let Some(l) = line_iter.peek() {
                if l.trim().is_empty() {
                    line_iter.next();
                } else {
                    break;
                }
            }
        } else if let Some(offset) = starts_with_digits(line.trim()) {
            output += "<ol><li>";
            output += &line.trim()[offset..];

            while let Some(line) = line_iter.next() {
                if line.trim().is_empty() {
                    break;
                }

                if let Some(offset) = starts_with_digits(line.trim()) {
                    output += "</li>\n<li>";
                    output += &line.trim()[offset..];
                } else {
                    output += "\n";
                    output += line.trim();
                }
            }

            output += "</li></ol>\n";
        } else if line.trim().starts_with("- ") {
            output += "<ul><li>";
            output += &line.trim()[2..];

            while let Some(line) = line_iter.next() {
                if line.trim().is_empty() {
                    break;
                }

                if line.trim().starts_with("- ") {
                    output += "</li>\n<li>";
                    output += &line.trim()[2..];
                } else {
                    output += "\n";
                    output += line.trim();
                }
            }

            output += "</li></ul>\n";
        } else {
            output += "<p>";
            output += line;

            while let Some(line) = line_iter.next() {
                if line.trim().is_empty() {
                    break;
                }
                output += "\n";
                output += line;
            }

            output += "</p>\n";
        }
    }

    output
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
        line[prefix_length..].trim(),
        prefix_length
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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

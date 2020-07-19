#[derive(Debug, PartialEq)]
pub struct HTMLElement {
    name: String,
    attributes: Vec<(String, String)>,
    tag_name: String,
    constructor: String,
    mutators: Vec<(Vec<String>, String)>,
    children: Vec<HTMLElement>,
    self_closing: bool,
    inner: String,
}

impl HTMLElement {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            attributes: Vec::new(),
            tag_name: String::new(),
            constructor: String::new(),
            mutators: Vec::new(),
            children: Vec::new(),
            self_closing: false,
            inner: String::new(),
        }
    }
}

pub fn parse(html: &str) -> Result<Vec<HTMLElement>, String> {
    let mut chars = html.chars().peekable();
    let mut output = Vec::new();
    while let Some(el) = take_one_element(&mut chars)? {
        output.push(el);
    }
    Ok(output)
}

pub fn take_one_element(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<Option<HTMLElement>, String> {
    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }

        if ch == '<' {
            if chars.peek() == Some(&'/') {
                while let Some(ch) = chars.next() {
                    if ch == '>' {
                        break;
                    }
                }
                break;
            }
            let mut element = read_tag(chars)?;

            if element.self_closing {
                return Ok(Some(element));
            }

            while let Some(child) = take_one_element(chars)? {
                element.children.push(child);
            }

            return Ok(Some(element));
        } else {
            let mut fragment = String::new();
            while let Some(ch) = chars.peek() {
                if *ch == '<' {
                    break;
                }
                fragment.push(*ch);
                chars.next();
            }

            if fragment.trim().is_empty() {
                continue;
            }

            let mut element = HTMLElement::new();
            element.tag_name = String::from("__fragment");
            element.inner = fragment.trim().to_string();
            return Ok(Some(element));
        }
    }
    Ok(None)
}

pub fn read_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut output = String::new();
    let mut termchar = '"';
    let mut quoted = false;
    let mut escaped = false;
    let mut started = false;
    while let Some(ch) = chars.next() {
        if !started && ch == '\'' || ch == '"' {
            termchar = ch;
            quoted = true;
            continue;
        }
        if !started && ch == '{' {
            quoted = true;
            termchar = '}';
            output.push('{');
            continue;
        }
        started = true;

        if !quoted && !ch.is_alphanumeric() {
            break;
        }

        if !escaped && ch == termchar {
            if termchar == '}' {
                output.push('}');
            }
            break;
        }

        if ch == '\\' {
            escaped = true;
        } else {
            escaped = false;
        }

        output.push(ch);
    }
    output
}

pub fn read_tag(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<HTMLElement, String> {
    let mut started_tag_name = false;
    let mut element = HTMLElement::new();

    // Read the name of the tag
    while let Some(ch) = chars.peek() {
        if !started_tag_name && ch.is_whitespace() {
            chars.next();
            continue;
        } else if started_tag_name && (ch.is_whitespace() || *ch == '>') {
            break;
        }

        element.tag_name.push(*ch);
        started_tag_name = true;
        chars.next();
    }

    // Read off all attributes
    let mut started_attr = false;
    while let Some(ch) = chars.peek() {
        if *ch == '/' {
            element.self_closing = true;
            chars.next();
            chars.next();
            break;
        }

        if *ch == '>' {
            chars.next();
            break;
        }

        if !started_attr && ch.is_whitespace() {
            chars.next();
            continue;
        }

        if started_attr && ch.is_whitespace() {
            started_attr = false;
            chars.next();
            continue;
        }

        if started_attr && *ch == '=' {
            chars.next();
            let len = element.attributes.len();
            element.attributes[len - 1].1 = read_string(chars);
            break;
        }

        if !started_attr {
            started_attr = true;
            element.attributes.push((String::new(), String::new()));
        }

        let len = element.attributes.len();
        element.attributes[len - 1].0.push(*ch);
        chars.next();
    }

    Ok(element)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing() {
        let result = parse("<p attr={x}><div tag='hey dude'>my stuff</div><br /></p>").unwrap();
        assert_eq!(result[0].tag_name, "p");
        assert_eq!(result[0].children[0].tag_name, "div");
    }
}

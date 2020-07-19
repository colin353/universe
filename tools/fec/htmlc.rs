#[derive(Debug, PartialEq)]
pub struct HTMLElement {
    pub name: String,
    attributes: Vec<(String, String)>,
    tag_name: String,
    constructor: String,
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
            children: Vec::new(),
            self_closing: false,
            inner: String::new(),
        }
    }

    pub fn to_js(&self) -> String {
        let mut output = self.define_element();
        output.push_str(&self.set_attributes());

        if self.children.len() > 0 {
            output.push('\n');
        }

        if !self.inner.is_empty() {
            output.push_str(&self.set_inner());
        }

        for child in &self.children {
            output.push_str(&child.to_js());
            output.push_str(&format!("{}.appendChild({})\n\n", self.name, child.name));
        }

        output
    }

    fn set_inner(&self) -> String {
        format!("{}.innerHTML = `{}`;\n", self.name, self.inner)
    }

    fn define_element(&self) -> String {
        format!(
            "const {} = document.createElement('{}');\n",
            self.name, self.tag_name
        )
    }

    fn set_attributes(&self) -> String {
        let mut output = String::new();
        for (k, v) in &self.attributes {
            output.push_str(&format!("{}.setAttribute('{}', '{}');\n", self.name, k, v));
        }
        output
    }

    fn extract_mutators(&self, mutators: &mut Vec<Mutator>) {
        // Check if there is any mutator needed for inner
        let deps = parse_fmtstring(&self.inner);
        if deps.len() > 0 {
            mutators.push(Mutator {
                inputs: deps,
                operation: format!("{}.innerHTML = `{}`", self.name, self.inner),
            });
        }

        for attr in &self.attributes {
            if attr.1.starts_with("{") && attr.1.ends_with("}") {
                let expr = &attr.1[1..attr.1.len() - 2];
                mutators.push(Mutator {
                    inputs: vec![expr.to_string()],
                    operation: format!("{}.setAttribute('{}', {});", self.name, attr.0, expr),
                });
            }

            let deps = parse_fmtstring(&attr.1);
            if deps.len() > 0 {
                mutators.push(Mutator {
                    inputs: deps,
                    operation: format!("{}.setAttribute('{}', `{}`);", self.name, attr.0, attr.1),
                });
            }
        }
    }

    pub fn get_mutators(&self) -> Vec<Mutator> {
        let mut output = Vec::new();
        self.extract_mutators(&mut output);
        for child in &self.children {
            child.extract_mutators(&mut output);
        }
        output
    }
}

#[derive(Debug, PartialEq)]
pub struct Mutator {
    pub inputs: Vec<String>,
    pub operation: String,
}

fn parse_fmtstring(fmt: &str) -> Vec<String> {
    let mut output = Vec::new();
    for (idx, _) in fmt.match_indices("${") {
        let substr = &fmt[idx + 2..];
        if let Some(end) = substr.find("}") {
            for dep in parse_expression(&fmt[idx + 2..(end + idx + 2)]) {
                output.push(dep);
            }
        }
    }
    output
}

fn parse_expression(expr: &str) -> Vec<String> {
    // TODO: more correctly implement this
    if expr.trim().is_empty() {
        return Vec::new();
    }

    vec![expr.trim().to_string()]
}

pub fn parse(html: &str) -> Result<Vec<HTMLElement>, String> {
    let mut chars = html.chars().peekable();
    let mut output = Vec::new();
    while let Some(el) = take_one_element(&mut chars)? {
        output.push(el);
    }

    let mut idx = 0;
    for el in output.iter_mut() {
        name_elements(el, &mut idx);
    }

    Ok(output)
}

fn name_element(idx: u64) -> String {
    format!("__el{}", idx)
}

fn name_elements(el: &mut HTMLElement, idx: &mut u64) {
    el.name = name_element(*idx);
    *idx += 1;
    for child in el.children.iter_mut() {
        name_elements(child, idx);
    }
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
            fragment.push(ch);
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
            element.tag_name = String::from("span");
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
    while let Some(ch) = chars.peek() {
        if !started && (*ch == '\'' || *ch == '"') {
            termchar = *ch;
            started = true;
            quoted = true;
            chars.next();
            continue;
        }
        if !started && *ch == '{' {
            quoted = true;
            termchar = '}';
            output.push('{');
            started = true;
            chars.next();
            continue;
        }
        started = true;

        if !quoted && !ch.is_alphanumeric() {
            break;
        }

        if !escaped && *ch == termchar {
            if termchar == '}' {
                output.push('}');
            }
            chars.next();
            break;
        }

        if *ch == '\\' {
            escaped = true;
        } else {
            escaped = false;
        }

        output.push(*ch);
        chars.next();
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
            started_attr = false;
            continue;
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
        let result = parse("<p><div tag=1>my stuff</div><br /></p>").unwrap();
        assert_eq!(result[0].name, "__el0");
        assert_eq!(result[0].tag_name, "p");
        assert_eq!(result[0].children[0].name, "__el1");
        assert_eq!(result[0].children[0].tag_name, "div");
        assert_eq!(result[0].children[0].children[0].inner, "my stuff");
    }

    #[test]
    fn test_parsing_2() {
        let result = parse("<div>my stuff</div>").unwrap();
        assert_eq!(result[0].tag_name, "div");
        assert_eq!(result[0].children[0].tag_name, "span");
        assert_eq!(result[0].children[0].inner, "my stuff");
    }

    #[test]
    fn test_parsing_3() {
        let result = parse("<p style=\"color: red\">red text</p>").unwrap();
        assert_eq!(result[0].name, "__el0");
        assert_eq!(result[0].tag_name, "p");
        assert_eq!(result[0].attributes[0].0, "style");
        assert_eq!(result[0].attributes[0].1, "color: red");
        assert_eq!(result[0].children[0].inner, "red text");
    }

    #[test]
    fn test_fmtstring_parsing() {
        let result = parse_fmtstring("test ${x} content");
        assert_eq!(result, vec!["x"]);
    }

    #[test]
    fn test_parse_expression() {
        let result = parse_expression("x");
        assert_eq!(result, vec!["x"]);
    }

    #[test]
    fn test_mutator_extraction() {
        let result = parse("<p>test ${x} content</p>").unwrap();
        assert_eq!(result[0].tag_name, "p");
        assert_eq!(result[0].get_mutators().len(), 1);
    }
}

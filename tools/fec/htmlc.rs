#[derive(Debug, PartialEq)]
pub enum HTMLAttributeKind {
    Quoted,
    Raw,
}

#[derive(Debug, PartialEq)]
pub struct HTMLAttribute {
    pub key: String,
    pub value: String,
    pub kind: HTMLAttributeKind,
}

#[derive(Debug, PartialEq)]
pub struct HTMLElement {
    pub name: String,
    prefix: String,
    pub attributes: Vec<HTMLAttribute>,
    tag_name: String,
    constructor: String,
    pub children: Vec<HTMLElement>,
    self_closing: bool,
    inner: String,
}

#[derive(Debug, PartialEq)]
pub enum ControlStatement {
    ForEach(String, String),
    Condition(String),
    Noop,
}

impl HTMLElement {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            prefix: String::new(),
            attributes: Vec::new(),
            tag_name: String::new(),
            constructor: String::new(),
            children: Vec::new(),
            self_closing: false,
            inner: String::new(),
        }
    }

    pub fn to_js(&mut self, parent_name: &str, add_dom_condition: &str) -> String {
        if self.tag_name == "control" {
            return self.to_js_ctrl(parent_name);
        }

        let mut output = self.define_element();
        output.push_str(&self.set_attributes());

        if self.children.len() > 0 {
            output.push('\n');
        }

        if !self.inner.is_empty() {
            output.push_str(&self.set_inner());
        }

        for child in &mut self.children {
            output.push_str(&child.to_js(&self.name, "true"));
        }

        output.push_str(&format!(
            r#"
            if({cond}) {{
                {parent_name}.appendChild({prefix}{name});
            }}
            "#,
            cond = add_dom_condition,
            parent_name = parent_name,
            prefix = self.prefix,
            name = self.name
        ));
        output
    }

    pub fn to_js_ctrl(&mut self, parent_name: &str) -> String {
        let mut output = String::new();
        match self.parse_control() {
            ControlStatement::Noop => {
                for child in &mut self.children {
                    output.push_str(&child.to_js(parent_name, "true"));
                    output.push_str(&format!(
                        "{}.appendChild({}{})\n\n",
                        parent_name, child.prefix, child.name
                    ));
                }
            }
            ControlStatement::ForEach(array, item) => {
                for child in &mut self.children {
                    output.push_str(&format!(
                        "const {}{}_elements = {{}};",
                        self.prefix, self.name
                    ));
                    output.push_str(&format!(
                        "for(const {}__key of Object.keys({})) {{\n",
                        self.name, array
                    ));
                    output.push_str(&format!(
                        "{}{}_elements[{}__key] = {{}};\n",
                        self.prefix, self.name, self.name,
                    ));
                    output.push_str(&format!(
                        "const {} = {}[{}__key];\n",
                        item, array, self.name
                    ));
                    output.push_str(&format!("const key = {}__key;\n", self.name));

                    child.set_prefix(format!(
                        "{}{}_elements[{}__key].",
                        self.prefix, self.name, self.name
                    ));

                    output.push_str(&child.to_js(parent_name, "true"));
                    output.push_str("}\n");
                }
            }
            ControlStatement::Condition(cond) => {
                for child in &mut self.children {
                    output.push_str(&child.to_js(parent_name, &cond));
                }
            }
        }
        output
    }

    pub fn set_prefix(&mut self, prefix: String) {
        for child in &mut self.children {
            child.set_prefix(prefix.clone());
        }
        self.prefix = prefix;
    }

    pub fn parse_control(&self) -> ControlStatement {
        // First, determine what kind of control statement it is
        for attr in &self.attributes {
            if attr.key == "for" {
                let mut item = String::from("item");
                for item_attr in &self.attributes {
                    if item_attr.key == "item" {
                        item = item_attr.key.clone();
                    }
                }

                return ControlStatement::ForEach(attr.value.clone(), item);
            }

            if attr.key == "if" {
                return ControlStatement::Condition(attr.value.clone());
            }
        }

        ControlStatement::Noop
    }

    fn set_inner(&self) -> String {
        format!("{}.innerHTML = `{}`;\n", self.name, self.inner)
    }

    fn define_element(&self) -> String {
        let mut output = format!(
            "const {} = document.createElement('{}');\n",
            self.name, self.tag_name
        );

        if !self.prefix.is_empty() {
            output.push_str(&format!("{}{} = {};\n", self.prefix, self.name, self.name));
        }

        output
    }

    fn set_attributes(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "{}{}.__rawAttributes ||= {{}}\n",
            self.prefix, self.name,
        ));

        for attr in &self.attributes {
            let k = &attr.key;
            let v = &attr.value;

            if k.starts_with("on:") {
                let event = &k[3..];
                let mut callback = v.as_str();
                if callback.starts_with("{") {
                    callback = &callback[1..];
                }
                if callback.ends_with("}") {
                    callback = &callback[..callback.len() - 2];
                }
                output.push_str(&format!(
                    "{}{}.addEventListener('{}', {}.bind(this));\n",
                    self.prefix, self.name, event, callback
                ));
            } else if k == "ref" {
                continue;
            } else {
                if attr.kind == HTMLAttributeKind::Quoted {
                    output.push_str(&format!(
                        "this.setElementAttribute({}{}, '{}', `{}`);\n",
                        self.prefix, self.name, k, v
                    ));
                } else {
                    output.push_str(&format!(
                        "this.setElementAttribute({}{}, '{}', {});\n",
                        self.prefix, self.name, k, v
                    ));
                }
            }
        }
        output
    }

    fn extract_mutators(&mut self, parent_name: &str, mutators: &mut Vec<Mutator>) {
        if self.tag_name == "control" {
            return self.extract_mutators_ctrl(parent_name, mutators);
        }

        // Check if there is any mutator needed for inner
        let deps = parse_fmtstring(&self.inner);
        if deps.len() > 0 {
            mutators.push(Mutator {
                inputs: deps,
                operation: format!("{}{}.innerHTML = `{}`", self.prefix, self.name, self.inner),
            });
        }

        for attr in &self.attributes {
            if attr.kind == HTMLAttributeKind::Quoted {
                let deps = parse_fmtstring(&attr.value);
                if deps.len() > 0 {
                    mutators.push(Mutator {
                        inputs: deps,
                        operation: format!(
                            "this.setElementAttribute({}{}, '{}', `{}`);",
                            self.prefix, self.name, attr.key, attr.value
                        ),
                    });
                }
            } else if attr.value.starts_with("this.state.") || attr.value.starts_with("item") {
                mutators.push(Mutator {
                    inputs: vec![attr.value.to_owned()],
                    operation: format!(
                        "this.setElementAttribute({prefix}{name}, '{key}', {value});\n",
                        prefix = self.prefix,
                        name = self.name,
                        key = attr.key,
                        value = attr.value,
                    ),
                });
            }
        }

        for child in &mut self.children {
            child.extract_mutators(&self.name, mutators);
        }
    }

    fn extract_mutators_ctrl(&mut self, parent_name: &str, mutators: &mut Vec<Mutator>) {
        match self.parse_control() {
            ControlStatement::Noop => {
                for child in &mut self.children {
                    child.extract_mutators(parent_name, mutators);
                }
            }
            ControlStatement::Condition(cond) => {
                for child in &mut self.children {
                    mutators.push(Mutator {
                        inputs: extract_dependencies(&cond),
                        operation: format!(
                            r#"
                            if({cond}) {{
                                {parent_name}.appendChild({prefix}{name});
                            }} else {{
                                {prefix}{name}.remove();
                            }}
                        "#,
                            cond = cond,
                            parent_name = parent_name,
                            prefix = child.prefix,
                            name = child.name
                        ),
                    });

                    child.extract_mutators(parent_name, mutators);
                }
            }
            ControlStatement::ForEach(array, item) => {
                let mut removals = String::new();
                for child in &self.children {
                    removals.push_str(&format!(
                        "{prefix}{name}_elements[{name}__key].{child_name}.remove();\n",
                        prefix = self.prefix,
                        name = self.name,
                        child_name = child.name,
                    ));
                }

                // Remove old entries mutator
                let op = format!(
                    r#"
                    for(const {name}__key of Object.keys({prefix}{name}_elements)) {{
                        if(!{array}[{name}__key]) {{
                            {removals}
                            delete {prefix}{name}_elements[{name}__key];
                        }}
                    }}"#,
                    name = self.name,
                    array = array,
                    prefix = self.prefix,
                    removals = removals,
                );

                mutators.push(Mutator {
                    inputs: vec![array.clone()],
                    operation: op,
                });

                let mut child_mutators = Vec::new();
                for child in &mut self.children {
                    child.extract_mutators(parent_name, &mut child_mutators);
                }

                for mut mutator in child_mutators {
                    mutator.operation = format!(
                        "for(const {}__key of Object.keys({}{}_elements)) {{const {} = {}[{}__key];\n{}\n}}",
                        self.name, self.prefix, self.name, item, array, self.name, mutator.operation, 
                    );

                    let mut array_dep = false;
                    for input in &mutator.inputs {
                        if input == &item || input.starts_with(&format!("{}.", item)) {
                            array_dep = true;
                            break;
                        }
                    }
                    if array_dep {
                        mutator.inputs.push(array.clone());
                    }

                    mutators.push(mutator);
                }

                let mut definitions = String::new();
                for child in &mut self.children {
                    definitions.push_str(&child.to_js(&parent_name, "true"));
                    definitions.push_str("\n\n");
                }

                // New entry mutator with additional comment
                let op = format!(
                    r#"
                        for(const {name}__key of Object.keys({array})) {{
                            if(!{prefix}{name}_elements[{name}__key]) {{
                                const key = {name}__key;
                                const {item} = {array}[{name}__key];
                                {prefix}{name}_elements[{name}__key] = {{}};
                                {definitions}
                            }}
                        }}"#,
                    name = self.name,
                    array = array,
                    prefix = self.prefix,
                    item = item,
                    definitions = definitions
                );

                mutators.push(Mutator {
                    inputs: vec![array.clone()],
                    operation: op,
                });
            }
        }
    }

    pub fn get_mutators(&mut self, parent_name: &str) -> Vec<Mutator> {
        let mut output = Vec::new();
        self.extract_mutators(parent_name, &mut output);
        output
    }
}

#[derive(Debug, PartialEq)]
pub struct Mutator {
    pub inputs: Vec<String>,
    pub operation: String,
}

fn normalize_dependency(dep: &str) -> &str {
    if dep.starts_with("this.state.") {
        if let Some(idx) = dep[11..].find(".") {
            return &dep[..11 + idx];
        }
    }

    dep
}

fn extract_dependencies(input: &str) -> Vec<String> {
    let mut output = Vec::new();
    for (idx, _) in input.match_indices("this.state.") {
        let dep: String = input
            .chars()
            .skip(idx)
            .take_while(|x| return x.is_alphanumeric() || *x == '.' || *x == '_')
            .collect();
        output.push(normalize_dependency(&dep).to_owned());
    }
    output
}

fn parse_fmtstring(fmt: &str) -> Vec<String> {
    let mut output = Vec::new();
    for (idx, _) in fmt.match_indices("${") {
        let substr = &fmt[idx + 2..];
        if let Some(end) = substr.find("}") {
            for dep in parse_expression(&fmt[idx + 2..(end + idx + 2)]) {
                output.push(dep.to_owned());
            }
        }
    }
    output
}

fn parse_expression(expr: &str) -> Vec<String> {
    expr.split(|c: char| !(c.is_alphanumeric() || c == '.' || c == '_'))
        .map(|x| normalize_dependency(x.trim()).to_string())
        .filter(|x| !x.is_empty())
        .collect()
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

pub fn read_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> (String, bool) {
    let mut output = String::new();
    let mut termchar = '"';
    let mut quoted = false;
    let mut escaped = false;
    let mut started = false;
    let mut raw = true;
    while let Some(ch) = chars.peek() {
        if !started && (*ch == '\'' || *ch == '"') {
            termchar = *ch;
            started = true;
            quoted = true;
            raw = false;
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

    (output, raw)
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
            let (value, raw) = read_string(chars);
            element.attributes[len - 1].value = value;

            if !raw {
                element.attributes[len - 1].kind = HTMLAttributeKind::Quoted;
            }

            started_attr = false;
            continue;
        }

        if !started_attr {
            started_attr = true;
            let attr = HTMLAttribute {
                key: String::new(),
                value: String::new(),
                kind: HTMLAttributeKind::Raw,
            };
            element.attributes.push(attr);
        }

        let len = element.attributes.len();
        element.attributes[len - 1].key.push(*ch);
        chars.next();
    }

    for attr in &mut element.attributes {
        // Strip off enclosing braces
        if attr.value.starts_with("{") && attr.value.ends_with("}") {
            attr.value = attr.value[1..attr.value.len() - 1].to_string();
        }
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
        assert_eq!(result[0].attributes[0].key, "style");
        assert_eq!(result[0].attributes[0].value, "color: red");
        assert_eq!(result[0].children[0].inner, "red text");
    }

    #[test]
    fn test_parsing_4() {
        let result = parse("<p on:click={fn}>red text</p>").unwrap();
        assert_eq!(result[0].name, "__el0");
        assert_eq!(result[0].tag_name, "p");
        assert_eq!(result[0].attributes[0].key, "on:click");
        assert_eq!(result[0].attributes[0].value, "fn");
        assert_eq!(result[0].children[0].inner, "red text");
    }

    #[test]
    fn test_fmtstring_parsing() {
        let result = parse_fmtstring("test ${x} content");
        assert_eq!(result, vec!["x"]);
    }

    #[test]
    fn test_fmtstring_parsing_2() {
        let result = parse_fmtstring("top: ${this.state.menuY}px; left: ${this.state.menuX}px");
        assert_eq!(result, vec!["this.state.menuY", "this.state.menuX"]);

        let result = parse_fmtstring("${this.state.test.length}");
        assert_eq!(result, vec!["this.state.test"]);
    }

    #[test]
    fn test_parse_expression() {
        let result = parse_expression("x");
        assert_eq!(result, vec!["x"]);

        let result = parse_expression("x + y");
        assert_eq!(result, vec!["x", "y"]);

        let result = parse_expression("JSON.parse(this.state.test)");
        assert_eq!(result, vec!["JSON.parse", "this.state.test"]);

        let result = parse_expression("this.state.test.length + 1");
        assert_eq!(result, vec!["this.state.test", "1"]);
    }

    #[test]
    fn test_mutator_extraction() {
        let mut result = parse("<p>test ${x} content</p>").unwrap();
        assert_eq!(result[0].tag_name, "p");
        assert_eq!(result[0].get_mutators("x").len(), 1);
    }

    #[test]
    fn test_extract_deps() {
        let deps = extract_dependencies("this.state.x > this.state.y");
        assert_eq!(deps[0], "this.state.x");
        assert_eq!(deps[1], "this.state.y");
    }
}

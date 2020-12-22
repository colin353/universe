use std::collections::HashMap;
use std::convert::From;

#[derive(Clone)]
pub struct ContentsMap(HashMap<&'static str, Contents>);

impl ContentsMap {
    pub fn new(data: HashMap<&'static str, Contents>) -> Self {
        Self(data)
    }

    pub fn get(&self, key: &str) -> Option<&Contents> {
        self.0.get(key)
    }

    pub fn get_str(&self, key: &str) -> String {
        if let Some(Contents::Value(x)) = self.0.get(key) {
            return x.to_owned();
        }
        String::new()
    }

    pub fn insert<T: Into<Contents>>(&mut self, key: &'static str, data: T) {
        self.0.insert(key, data.into());
    }

    fn resolve(&self, key: &str) -> Option<&Contents> {
        let idx = match key.find(".") {
            Some(idx) => idx,
            None => return self.get(key),
        };

        let component = &key[0..idx];
        let remaining_path = &key[idx + 1..];
        match self.get(component) {
            Some(Contents::Object(m)) => m.resolve(remaining_path),
            x => x,
        }
    }
}

#[derive(Clone)]
pub enum Contents {
    Value(String),
    MultiValue(ContentsMultiMap),
    Object(ContentsMap),
}

#[derive(Clone)]
pub struct ContentsMultiMap {
    data: Vec<ContentsMap>,
}

impl From<Vec<ContentsMap>> for ContentsMultiMap {
    fn from(input: Vec<ContentsMap>) -> Self {
        ContentsMultiMap { data: input }
    }
}

impl ContentsMultiMap {
    pub fn new(data: Vec<ContentsMap>) -> Self {
        ContentsMultiMap { data: data }
    }
}

impl<T: ToString> From<T> for Contents {
    fn from(input: T) -> Self {
        let value: String = input.to_string();
        Contents::Value(value)
    }
}

impl From<ContentsMultiMap> for Contents {
    fn from(input: ContentsMultiMap) -> Self {
        Contents::MultiValue(input)
    }
}

impl From<ContentsMap> for Contents {
    fn from(input: ContentsMap) -> Self {
        Contents::Object(input)
    }
}

#[derive(Debug, PartialEq)]
pub enum Key<'a> {
    Value(&'a str),
    MultiValue(&'a str),
    EqualityCondition(&'a str, &'a str),
    InequalityCondition(&'a str, &'a str),
    CloseBlock(&'a str),
}

pub fn apply(template: &str, data: &ContentsMap) -> String {
    let mut out = String::from("");
    apply_mut(template, data, &mut out);
    out
}

pub fn apply_with_settings(
    template: &str,
    mut data: ContentsMap,
    settings: &ContentsMap,
) -> String {
    let mut out = String::from("");
    data.insert("settings", Contents::Object(settings.clone()));
    apply_mut(template, &data, &mut out);
    out
}

pub fn apply_mut(template: &str, data: &ContentsMap, output: &mut String) {
    let mut parser = Parser::new(template);
    while let Some((start, end, maybe_key)) = parser.next() {
        output.push_str(&parser.template[start..end]);

        let key = if let Some(key) = maybe_key {
            key
        } else {
            // If there's no key, it means there are no following keys, so we can just push the
            // last content in and finish.
            continue;
        };

        match decode_key(key) {
            // Regular key insertion.
            Key::Value(key) => match data.resolve(key) {
                Some(Contents::Value(x)) => output.push_str(&x),
                _ => (),
            },

            // Array insertion.
            Key::MultiValue(key) => match data.resolve(key) {
                Some(Contents::MultiValue(x)) => {
                    let loop_template = parser.jump_to_close_tag(key);
                    // Sometimes the inner loop wants to access the parent state. So just clone
                    // the parent state and merge the element state on top.
                    let mut cloned_data = data.clone();
                    for value in &x.data {
                        for (k, v) in value.0.iter() {
                            cloned_data.0.insert(k, (*v).clone());
                        }
                        apply_mut(loop_template, &cloned_data, output);
                    }
                }
                _ => (),
            },

            // Equality condition, only render the block if the condition is true.
            Key::EqualityCondition(key, value) => {
                let block_template = parser.jump_to_close_tag(key);

                // If it's a multi variable, equality tests for the length of the array.
                let key = if key.ends_with("[]") {
                    let (variable, _) = key.split_at(key.len() - 2);
                    variable.trim()
                } else {
                    key
                };

                match data.resolve(key) {
                    Some(Contents::Value(x)) => {
                        if x == value {
                            apply_mut(block_template, data, output);
                        }
                    }
                    Some(Contents::MultiValue(x)) => {
                        let length: usize = match value.parse() {
                            Ok(length) => length,
                            Err(_) => {
                                eprintln!("equality condition with multi-value key {}, cannot parse int {}", key, value);
                                continue;
                            }
                        };

                        if x.data.len() == length {
                            apply_mut(block_template, data, output);
                        }
                    }

                    _ => eprintln!("equality condition key {} not found", key),
                }
            }

            // Inequality condition, only render the block if the condition is false.
            Key::InequalityCondition(key, value) => {
                let block_template = parser.jump_to_close_tag(key);

                // If it's a multi variable, equality tests for the length of the array.
                let key = if key.ends_with("[]") {
                    let (variable, _) = key.split_at(key.len() - 2);
                    variable.trim()
                } else {
                    key
                };

                match data.resolve(key) {
                    Some(Contents::Value(x)) => {
                        if x != value {
                            apply_mut(block_template, data, output);
                        }
                    }
                    Some(Contents::MultiValue(x)) => {
                        let length: usize = match value.parse() {
                            Ok(length) => length,
                            Err(_) => {
                                eprintln!("inequality condition with multi-value key {}, cannot parse int {}", key, value);
                                continue;
                            }
                        };

                        if x.data.len() != length {
                            apply_mut(block_template, &data, output);
                        }
                    }

                    _ => eprintln!("inequality condition key {} not found", key),
                }
            }

            // If we observe a close block, that's an invalid template.
            Key::CloseBlock(key) => {
                eprintln!("invalid closing block: {}", key);
            }
        }
    }
}

fn decode_key<'a>(key: &'a str) -> Key<'a> {
    // Remove any extraneous whitespace.
    let key = key.trim();

    // Check if it starts with a slash, then it's a close block.
    if key.starts_with("/") {
        return Key::CloseBlock(&key[1..]);
    }

    // Check if it is an equality condition.
    if let Some(idx) = key.find("==") {
        let (variable, _) = key.split_at(idx);
        let (_, value) = key.split_at(idx + 2);

        // Remove the whitespace around the variable.
        let variable = variable.trim();

        // Remove the quotes and whitespace around the comparison value.
        let value = value.trim().trim_matches('"').trim_matches('\'');

        return Key::EqualityCondition(variable, value);
    }

    // Check if it is an inequality condition.
    if let Some(idx) = key.find("!=") {
        let (variable, _) = key.split_at(idx);
        let (_, value) = key.split_at(idx + 2);

        // Remove the whitespace around the variable.
        let variable = variable.trim();

        // Remove the quotes and whitespace around the comparison value.
        let value = value.trim().trim_matches('"').trim_matches('\'');

        return Key::InequalityCondition(variable, value);
    }

    // Check for multi-value variable.
    if key.ends_with("[]") {
        let (variable, _) = key.split_at(key.len() - 2);

        // Remove the whitespace around the variable.
        let variable = variable.trim();

        return Key::MultiValue(variable);
    }

    Key::Value(key)
}

struct Parser<'a> {
    index: usize,
    template: &'a str,
}

impl<'a> Parser<'a> {
    fn new(template: &'a str) -> Self {
        Parser {
            index: 0,
            template: template,
        }
    }

    fn jump_to_close_tag(&mut self, key: &str) -> &'a str {
        let start = self.index;
        let mut depth = 0;
        loop {
            match self.next() {
                Some((_, end, Some(next_key))) => match decode_key(next_key) {
                    Key::MultiValue(a) if a == key => {
                        depth += 1;
                    }
                    Key::EqualityCondition(a, _) if a == key => {
                        depth += 1;
                    }
                    Key::InequalityCondition(a, _) if a == key => {
                        depth += 1;
                    }
                    Key::CloseBlock(a) if a == key => {
                        if depth == 0 {
                            return &self.template[start..end];
                        }
                        depth -= 1;
                    }
                    _ => continue,
                },
                _ => break,
            }
        }

        eprintln!("No matching close tag!");
        let start = self.index;
        self.index = self.template.len();
        return &self.template[start..];
    }

    fn next(&mut self) -> Option<(usize, usize, Option<&'a str>)> {
        let rest_of_template = &self.template[self.index..];
        if rest_of_template == "" {
            return None;
        }

        match rest_of_template.find("{{") {
            Some(key_start_idx) => {
                let rest = &rest_of_template[key_start_idx..];
                match rest.find("}}") {
                    Some(key_end_idx) => {
                        let key = &rest[2..key_end_idx];
                        let start = self.index;
                        self.index += key_start_idx + key_end_idx + 2;
                        return Some((start, start + key_start_idx, Some(key)));
                    }
                    None => {
                        eprintln!("No matching }}");
                        None
                    }
                }
            }
            None => {
                let start = self.index;
                let end = self.template.len();
                self.index = end;
                return Some((start, end, None));
            }
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = (usize, usize, Option<&'a str>);
    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

#[macro_export]
macro_rules! content {
    ($($key:expr => $value:expr),*) => {
        {
            let mut m = std::collections::HashMap::<&str, $crate::Contents>::new();
            $( m.insert($key, $value.into()); )*
            $crate::ContentsMap::new(m)
        }
    };
    ($($key:expr => $value:expr),*; $($key2:expr => $multivalue:expr),* ) => {
        {
            let mut m = std::collections::HashMap::<&str, $crate::Contents>::new();
            $( m.insert($key, $value.into()); )*
            $( m.insert($key2, $crate::ContentsMultiMap::new($multivalue).into()); )*
            $crate::ContentsMap::new(m)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macros() {
        let template = "hello, {{name}}, you are {{age}} years old!";
        assert_eq!(
            apply(
                template,
                &content!(
                    "name" => "Colin",
                    "age" => 300
                ),
            ),
            "hello, Colin, you are 300 years old!"
        );
    }

    #[test]
    fn test_recursive() {
        let template =
            "object: {{object.schloop}} {{object.candidates[]}}{{name}} {{/object.candidates}}";
        assert_eq!(
            apply(
                template,
                &content!(
                    "name" => "Colin",
                    "age" => 300,
                    "object" => content!(
                        "schloop" => "dwoop";
                        "candidates" => vec![content!("name" => "X"), content!("name" => "Y")]
                    )
                ),
            ),
            "object: dwoop X Y "
        );
    }

    #[test]
    fn test_key_decoding() {
        assert_eq!(decode_key("variable"), Key::Value("variable"));
        assert_eq!(
            decode_key("variable == \"value\""),
            Key::EqualityCondition("variable", "value")
        );

        assert_eq!(
            decode_key("variable != \" a more complex's value \""),
            Key::InequalityCondition("variable", " a more complex's value ")
        );

        assert_eq!(decode_key(" variable[] "), Key::MultiValue("variable"));
    }

    #[test]
    fn test_parser() {
        let mut p = Parser::new("Hello, {{name}}!");
        assert_eq!(p.next(), Some((0, 7, Some("name"))));
        assert_eq!(p.next(), Some((15, 16, None)));
        assert_eq!(p.next(), None);
    }

    #[test]
    fn test_jump_to_close_tag() {
        let mut p = Parser::new("Hello, {{values}}inner content{{/values}}!");
        assert_eq!(p.next(), Some((0, 7, Some("values"))));
        assert_eq!(p.jump_to_close_tag("values"), "inner content");
        assert_eq!(p.next(), Some((41, 42, None)));
        assert_eq!(p.next(), None);
    }

    #[test]
    fn test_apply() {
        let template = "Hello, {{name}}!";
        let contents = content!( "name" => "world");
        assert_eq!(apply(template, &contents), "Hello, world!");
    }

    #[test]
    fn test_apply_loop() {
        let template = "People:{{people[]}} {{name}}, {{title}}.{{/people}}";

        let contents = content!(;
            "people" => vec![
                content!(
                    "name" => "Colin",
                    "title" => "Tester"
                ),
                content!(
                    "name" => "John",
                    "title" => "Tester"
                )
            ]
        );

        let expected = "People: Colin, Tester. John, Tester.";

        assert_eq!(apply(template, &contents), expected, "Not equals");
    }

    #[test]
    fn test_apply_conditional() {
        let template = "Test... {{secret == true}}secret message{{/secret}}!";
        let mut contents = HashMap::new();
        contents.insert("secret", Contents::Value(String::from("true")));

        let expected = "Test... secret message!";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);

        let mut contents = HashMap::new();
        contents.insert("secret", Contents::Value(String::from("false")));

        let expected = "Test... !";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);
    }

    #[test]
    fn test_apply_false_conditional() {
        let template = "Test... {{secret != true}}secret message{{/secret}}!";
        let mut contents = HashMap::new();
        contents.insert("secret", Contents::Value(String::from("true")));

        let expected = "Test... !";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);

        let mut contents = HashMap::new();
        contents.insert("secret", Contents::Value(String::from("false")));

        let expected = "Test... secret message!";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);
    }

    #[test]
    fn test_apply_array_conditional() {
        let template = "{{array == 0}}No records found.{{/array}}";
        let mut contents = HashMap::new();
        contents.insert(
            "array",
            Contents::MultiValue(ContentsMultiMap::new(Vec::new())),
        );

        let expected = "No records found.";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);

        // Try doing it with some records.
        let mut contents = HashMap::new();
        let mut records = Vec::new();
        records.push(ContentsMap(HashMap::new()));
        contents.insert(
            "array",
            Contents::MultiValue(ContentsMultiMap::new(records)),
        );

        let expected = "";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);
    }

    #[test]
    fn test_apply_array_conditional_inequality() {
        let template = "{{array != 0}}Found some records!{{/array}}";
        let mut contents = HashMap::new();
        contents.insert(
            "array",
            Contents::MultiValue(ContentsMultiMap::new(Vec::new())),
        );

        let expected = "";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);

        // Try doing it with some records.
        let mut contents = HashMap::new();
        let mut records = Vec::new();
        records.push(ContentsMap(HashMap::new()));
        contents.insert(
            "array",
            Contents::MultiValue(ContentsMultiMap::new(records)),
        );

        let expected = "Found some records!";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);
    }

    #[test]
    fn test_apply_nested_conditions() {
        let template =
            "{{array != 0}}Found some records: [{{array[]}}{{name}} {{/array}}] for you!{{/array}}";

        let mut contents = HashMap::new();
        contents.insert(
            "array",
            Contents::MultiValue(ContentsMultiMap::new(Vec::new())),
        );

        let expected = "";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);

        // Try doing it with some records.
        let mut contents = HashMap::new();
        let mut records = Vec::new();
        records.push(content!(
            "name" => "Colin"
        ));
        records.push(content!(
            "name" => "Tim"
        ));

        contents.insert(
            "array",
            Contents::MultiValue(ContentsMultiMap::new(records)),
        );

        let expected = "Found some records: [Colin Tim ] for you!";
        assert_eq!(apply(template, &ContentsMap(contents)), expected);
    }

    #[test]
    fn test_use_parent_data() {
        let template = "id={{id}}, {{people[]}}{{name}}:{{id}} {{/people}}";

        let contents = content!(
            "id" => "4";
            "people" => vec![
                content!("name" => "Colin"),
                content!("name" => "Tim"),
            ]
        );

        let expected = "id=4, Colin:4 Tim:4 ";
        assert_eq!(apply(template, &contents), expected);
    }

    #[test]
    fn test_conditional() {
        let template = "{{a != \"\"}}{{a}}{{/a}}{{a == \"\"}}hello world{{/a}}";

        let contents = content!(
            "a" => "hi planet"
        );
        let expected = "hi planet";
        assert_eq!(apply(template, &contents), expected);

        let contents = content!(
            "a" => ""
        );
        let expected = "hello world";
        assert_eq!(apply(template, &contents), expected);
    }

    #[test]
    fn test_use_settings_data() {
        let template = "{{settings.test}} {{content}}";

        let contents = content!(
            "content" => "winner"
        );
        let settings = content!(
            "test" => "game"
        );
        let expected = "game winner";
        assert_eq!(apply_with_settings(template, contents, &settings), expected);
    }
}

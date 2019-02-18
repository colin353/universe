use std::collections::HashMap;
use std::convert::From;

pub enum Contents {
    Value(String),
    MultiValue(ContentsMultiMap),
}

pub struct ContentsMultiMap {
    data: Vec<HashMap<&'static str, Contents>>,
}

impl From<Vec<HashMap<&'static str, Contents>>> for ContentsMultiMap {
    fn from(input: Vec<HashMap<&'static str, Contents>>) -> Self {
        ContentsMultiMap { data: input }
    }
}

impl ContentsMultiMap {
    pub fn new(data: Vec<HashMap<&'static str, Contents>>) -> Self {
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

#[derive(Debug, PartialEq)]
pub enum Key<'a> {
    Value(&'a str),
    MultiValue(&'a str),
    EqualityCondition(&'a str, &'a str),
    InequalityCondition(&'a str, &'a str),
}

pub fn apply(template: &str, data: &HashMap<&str, Contents>) -> String {
    let mut out = String::from("");
    apply_mut(template, data, &mut out);
    out
}

pub fn apply_mut(template: &str, data: &HashMap<&str, Contents>, output: &mut String) {
    let mut parser = Parser::new(template);
    while let Some((start, maybe_key)) = parser.next() {
        output.push_str(start);

        let key = if let Some(key) = maybe_key {
            key
        } else {
            // If there's no key, it means there are no following keys, so we can just push the
            // last content in and finish.
            continue;
        };

        match decode_key(key) {
            // Regular key insertion.
            Key::Value(key) => match data.get(key) {
                Some(Contents::Value(x)) => output.push_str(&x),
                _ => eprintln!("key {} not found", key),
            },

            // Array insertion.
            Key::MultiValue(key) => match data.get(key) {
                Some(Contents::MultiValue(x)) => {
                    let loop_template = parser.jump_to_close_tag(key);
                    for value in &x.data {
                        apply_mut(loop_template, value, output);
                    }
                }
                _ => eprintln!("multi-value key {} not found", key),
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

                match data.get(key) {
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

                match data.get(key) {
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
        }
    }
}

fn decode_key<'a>(key: &'a str) -> Key<'a> {
    // Remove any extraneous whitespace.
    let key = key.trim();

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
    template: &'a str,
}

impl<'a> Parser<'a> {
    fn new(template: &'a str) -> Self {
        Parser { template: template }
    }

    fn jump_to_close_tag(&mut self, key: &str) -> &'a str {
        let close_tag = format!("{{{{/{}}}}}", key);
        match self.template.find(&close_tag) {
            Some(idx) => {
                let (inside, rest) = self.template.split_at(idx);
                let (_, rest) = rest.split_at(close_tag.len());
                self.template = rest;
                inside
            }
            None => {
                eprintln!("No matching close tag!");
                let rest = self.template;
                self.template = "";
                rest
            }
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = (&'a str, Option<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.template == "" {
            return None;
        }

        match self.template.find("{{") {
            Some(idx) => {
                let (start, rest) = self.template.split_at(idx);
                match rest.find("}}") {
                    Some(idx) => {
                        let (key, rest) = rest.split_at(idx);

                        // Remove the leading {{ from the key.
                        let (_, key) = key.split_at(2);
                        // Remove the leading }} from the rest.
                        let (_, rest) = rest.split_at(2);

                        self.template = rest;
                        return Some((start, Some(key)));
                    }
                    None => {
                        eprintln!("No matching }}");
                        None
                    }
                }
            }
            None => {
                let start = self.template;
                self.template = "";
                return Some((start, None));
            }
        }
    }
}

#[macro_export]
macro_rules! content {
    ($($key:expr => $value:expr),*) => {
        {
            let mut m = std::collections::HashMap::<&str, $crate::Contents>::new();
            $( m.insert($key, $value.into()); )*
            m
        }
    };
    ($($key:expr => $value:expr),*; $($key2:expr => $multivalue:expr)* ) => {
        {
            let mut m = std::collections::HashMap::<&str, $crate::Contents>::new();
            $( m.insert($key, $value.into()); )*
            $( m.insert($key2, ContentsMultiMap::new($multivalue).into()); )*
            m
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
        assert_eq!(p.next(), Some(("Hello, ", Some("name"))));
        assert_eq!(p.next(), Some(("!", None)));
        assert_eq!(p.next(), None);
    }

    #[test]
    fn test_jump_to_close_tag() {
        let mut p = Parser::new("Hello, {{values}}inner content{{/values}}!");
        assert_eq!(p.next(), Some(("Hello, ", Some("values"))));
        assert_eq!(p.jump_to_close_tag("values"), "inner content");
        assert_eq!(p.next(), Some(("!", None)));
        assert_eq!(p.next(), None);
    }

    #[test]
    fn test_apply() {
        let template = "Hello, {{name}}!";
        let mut contents = content!( "name" => "world");
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
        assert_eq!(apply(template, &contents), expected);

        let mut contents = HashMap::new();
        contents.insert("secret", Contents::Value(String::from("false")));

        let expected = "Test... !";
        assert_eq!(apply(template, &contents), expected);
    }

    #[test]
    fn test_apply_false_conditional() {
        let template = "Test... {{secret != true}}secret message{{/secret}}!";
        let mut contents = HashMap::new();
        contents.insert("secret", Contents::Value(String::from("true")));

        let expected = "Test... !";
        assert_eq!(apply(template, &contents), expected);

        let mut contents = HashMap::new();
        contents.insert("secret", Contents::Value(String::from("false")));

        let expected = "Test... secret message!";
        assert_eq!(apply(template, &contents), expected);
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
        assert_eq!(apply(template, &contents), expected);

        // Try doing it with some records.
        let mut contents = HashMap::new();
        let mut records = Vec::new();
        records.push(HashMap::new());
        contents.insert(
            "array",
            Contents::MultiValue(ContentsMultiMap::new(records)),
        );

        let expected = "";
        assert_eq!(apply(template, &contents), expected);
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
        assert_eq!(apply(template, &contents), expected);

        // Try doing it with some records.
        let mut contents = HashMap::new();
        let mut records = Vec::new();
        records.push(HashMap::new());
        contents.insert(
            "array",
            Contents::MultiValue(ContentsMultiMap::new(records)),
        );

        let expected = "Found some records!";
        assert_eq!(apply(template, &contents), expected);
    }
}

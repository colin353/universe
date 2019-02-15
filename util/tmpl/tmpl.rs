use std::collections::HashMap;

pub enum Contents {
    Value(String),
    MultiValue(Vec<HashMap<&'static str, Contents>>),
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

        if let Some(key) = maybe_key {
            match data.get(key) {
                Some(Contents::Value(x)) => output.push_str(&x),
                Some(Contents::MultiValue(x)) => {
                    let loop_template = parser.jump_to_close_tag(key);
                    for value in x {
                        apply_mut(loop_template, value, output);
                    }
                }
                None => eprintln!("Unable to find key `{}`!", key),
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut contents = HashMap::new();
        contents.insert("name", Contents::Value(String::from("world")));

        assert_eq!(apply(template, &contents), "Hello, world!", "Not equals");
    }

    #[test]
    fn test_apply_loop() {
        let template = "People:{{people}} {{name}}, {{title}}.{{/people}}";
        let mut people = Vec::new();
        let mut p = HashMap::new();
        p.insert("name", Contents::Value(String::from("Colin")));
        p.insert("title", Contents::Value(String::from("Tester")));
        people.push(p);

        let mut p = HashMap::new();
        p.insert("name", Contents::Value(String::from("John")));
        p.insert("title", Contents::Value(String::from("Tester")));
        people.push(p);

        let mut contents = HashMap::new();
        contents.insert("people", Contents::MultiValue(people));

        let expected = "People: Colin, Tester. John, Tester.";

        assert_eq!(apply(template, &contents), expected, "Not equals");
    }
}

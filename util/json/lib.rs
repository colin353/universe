// A JSON parser implemented using ggen
mod ast;

use std::collections::HashMap;

use ggen::{GrammarUnit, Whitespace};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Array(Vec<Value>),
    Dictionary(HashMap<String, Value>),
}

ggen::sequence!(
    JSON,
    _ws1: Option<Whitespace>,
    value: ast::JSONValue,
    _ws2: Option<Whitespace>,
);

pub fn parse(input: &str) -> std::io::Result<Value> {
    let value = match JSON::try_match(input, 0) {
        Ok((value, _)) => value,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unable to parse as JSON!",
            ))
        }
    };
    Ok(ast::convert(value.value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json() {
        let content = r#"
            { "a": true, "b": [true, false], "c": 3.14, "d": "qqq" }
            "#;
        let mut map = HashMap::new();
        map.insert(String::from("a"), Value::Boolean(true));
        map.insert(
            String::from("b"),
            Value::Array(vec![Value::Boolean(true), Value::Boolean(false)]),
        );
        map.insert(String::from("c"), Value::Number(3.14));
        map.insert(String::from("d"), Value::String(String::from("qqq")));
        let expected = Value::Dictionary(map);

        assert_eq!(parse(content).unwrap(), expected);
    }
}

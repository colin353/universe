// A JSON parser implemented using ggen
mod ast;

use std::collections::HashMap;

use ggen::{GrammarUnit, ParseError, Whitespace};

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

pub fn parse(input: &str) -> Result<Value, ParseError> {
    let value = match JSON::try_match(input, 0) {
        Ok((value, _, _)) => value,
        Err(err) => return Err(err),
    };
    Ok(ast::convert(value.value))
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_fail {
        ($content:expr, $expected:expr,) => {
            let fail = parse($content);
            assert!(fail.is_err());
            let got = fail.unwrap_err().render($content);
            if got.trim() != $expected.trim() {
                println!("got:\n\n{}\n", got.trim_matches('\n'));
                println!("expected:\n\n{}\n", $expected.trim_matches('\n'));
                panic!("got != expected");
            }
        };
    }

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

    #[test]
    fn parse_fail() {
        assert_fail!(
            r#"
{
    "a": true,
    "b": [true, false],
    3,
}
"#,
            r#"
   |
5  |     3,
   |     ^ expected one of: quoted string, }
"#,
        );

        assert_fail!(
            r#"
{
    "a": 3.141.59
}
"#,
            r#"
   |
3  |     "a": 3.141.59
   |          ^^^^^^^^ unable to parse number
"#,
        );

        assert_fail!(
            r#"
{
    "a": [true, faulse]
}
"#,
            r#"
   |
3  |     "a": [true, faulse]
   |                 ^ expected one of: number, quoted string, Boolean, null, Array, Dictionary, ]
"#,
        );
    }
}

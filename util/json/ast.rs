use std::collections::HashMap;

use crate::Value;

use ggen::{Numeric, QuotedString, RepeatWithSeparator, Whitespace};

ggen::one_of!(
    JSONValue,
    Number: Numeric,
    String: QuotedString,
    Boolean: Boolean,
    Null: Null,
    Array: Array,
    Dictionary: Dictionary
);

ggen::sequence!(
    Array,
    "[",
    _ws1: Option<Whitespace>,
    inner: Option<RepeatWithSeparator<JSONValue, CommaSeparator>>,
    _comma: Option<CommaSeparator>, // consume possibly trailing comma
    _ws2: Option<Whitespace>,
    "]",
);

ggen::unit!(Null, "null");
ggen::unit!(BooleanTrue, "true");
ggen::unit!(BooleanFalse, "false");
ggen::one_of!(Boolean, True: BooleanTrue, False: BooleanFalse);

ggen::sequence!(
    KVPair,
    key: QuotedString,
    _ws1: Option<Whitespace>,
    ":",
    _ws2: Option<Whitespace>,
    value: JSONValue,
);

ggen::sequence!(
    CommaSeparator,
    _ws1: Option<Whitespace>,
    ",",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    Dictionary,
    "{",
    _ws1: Option<Whitespace>,
    kv_pairs: Option<RepeatWithSeparator<KVPair, CommaSeparator>>,
    _comma: Option<CommaSeparator>,
    _ws2: Option<Whitespace>,
    "}",
);

pub fn convert(value: JSONValue) -> Value {
    match value {
        JSONValue::Number(num) => Value::Number(num.value),
        JSONValue::String(s) => Value::String(s.value),
        JSONValue::Boolean(b) => match *b {
            Boolean::True(_) => Value::Boolean(true),
            Boolean::False(_) => Value::Boolean(false),
        },
        JSONValue::Null(_) => Value::Null,
        JSONValue::Array(arr) => Value::Array(
            arr.inner
                .unwrap_or(RepeatWithSeparator::empty())
                .values
                .into_iter()
                .map(|v| convert(v))
                .collect(),
        ),
        JSONValue::Dictionary(dict) => {
            let mut output = HashMap::new();
            if let Some(pairs) = dict.kv_pairs {
                for pair in pairs.values {
                    output.insert(pair.key.value.to_string(), convert(pair.value));
                }
            }
            Value::Dictionary(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ggen::GrammarUnit;

    macro_rules! assert_range {
        ($unit:expr, $content:expr, $expected:expr,) => {
            let (start, end) = $unit.range();
            assert_eq!(
                $expected,
                format!("{}{}", " ".repeat(start), "^".repeat(end - start),)
            );
        };
    }

    #[test]
    fn test_dictionary_match() {
        assert!(JSONValue::try_match(r#"5"#, 0).is_ok());
        assert!(QuotedString::try_match(r#""abc""#, 0).is_ok());
        assert!(KVPair::try_match(r#""abc":5,"#, 0).is_ok());

        let (unit, _, _) = Dictionary::try_match(r#"{"abc": 5}"#, 0).unwrap();
        assert_range!(
            unit,
            r#"{"abc": 5}"#, //
            r#"^^^^^^^^^^"#,
        );

        assert_eq!(unit.kv_pairs.as_ref().unwrap().len(), 1);

        assert_range!(
            unit.kv_pairs.as_ref().unwrap().values[0],
            r#"{"abc": 5}"#, //
            r#" ^^^^^^^^"#,
        );

        assert_range!(
            unit.kv_pairs.as_ref().unwrap().values[0].key,
            r#"{"abc": 5}"#, //
            r#" ^^^^^"#,
        );

        assert_range!(
            unit.kv_pairs.as_ref().unwrap().values[0].value,
            r#"{"abc": 5}"#, //
            r#"        ^"#,
        );
    }

    #[test]
    fn test_dictionary_multi_value_match() {
        let (unit, _, _) = Dictionary::try_match(r#"{"abc": 5, "def": "aaa"}"#, 0).unwrap();
        assert_range!(
            unit,
            r#"{"abc": 5, "def": "aaa"}"#,
            r#"^^^^^^^^^^^^^^^^^^^^^^^^"#,
        );

        assert_eq!(unit.kv_pairs.as_ref().unwrap().len(), 2);

        assert_range!(
            unit.kv_pairs.as_ref().unwrap().values[0],
            r#"{"abc": 5, "def": "aaa"}"#,
            r#" ^^^^^^^^"#,
        );

        assert_range!(
            unit.kv_pairs.as_ref().unwrap().values[1],
            r#"{"abc": 5, "def": "aaa"}"#,
            r#"           ^^^^^^^^^^^^"#,
        );

        if let JSONValue::Number(num) = &unit.kv_pairs.as_ref().unwrap().values[0].value {
            assert_eq!(num.value, 5.0);
        } else {
            panic!(
                "value {:?} didn't match pattern!",
                unit.kv_pairs.unwrap().values[0].value
            );
        }

        if let JSONValue::String(s) = &unit.kv_pairs.as_ref().unwrap().values[1].value {
            assert_eq!(&s.value, "aaa");
        } else {
            panic!(
                "value {:?} didn't match pattern!",
                unit.kv_pairs.unwrap().values[1].value
            );
        }
    }

    #[test]
    fn test_recursive() {
        let (unit, _, _) = JSONValue::try_match(r#"{"abc": 5, "def": [1,2,3,4,5]}"#, 0).unwrap();
        if let JSONValue::Dictionary(dict) = unit {
            assert_range!(
                dict.kv_pairs.as_ref().unwrap().values[0],
                r#"{"abc": 5, "def": [1,2,3,4,5]}"#,
                r#" ^^^^^^^^"#,
            );

            assert_range!(
                dict.kv_pairs.as_ref().unwrap().values[1],
                r#"{"abc": 5, "def": [1,2,3,4,5]}"#,
                r#"           ^^^^^^^^^^^^^^^^^^"#,
            );

            assert_range!(
                dict.kv_pairs.as_ref().unwrap().values[1],
                r#"{"abc": 5, "def": [1,2,3,4,5]}"#,
                r#"           ^^^^^^^^^^^^^^^^^^"#,
            );

            assert_range!(
                dict.kv_pairs.as_ref().unwrap().values[1].value,
                r#"{"abc": 5, "def": [1,2,3,4,5]}"#,
                r#"                  ^^^^^^^^^^^"#,
            );

            if let JSONValue::Array(arr) = &dict.kv_pairs.as_ref().unwrap().values[1].value {
                assert_eq!(
                    arr.inner
                        .as_ref()
                        .unwrap_or(&RepeatWithSeparator::empty())
                        .values
                        .len(),
                    5
                );
            } else {
                panic!("wasn't an array!");
            }
        } else {
            panic!("wasn't a dictionary!");
        }
    }
}

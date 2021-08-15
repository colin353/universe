// A JSON parser implemented using ggen

use ggen::{Numeric, QuotedString, RepeatWithSeparator, Whitespace};

pub struct JSON {}

ggen::one_of!(
    JSONValue,
    Number: Numeric,
    String: QuotedString,
    Boolean: Boolean
);

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
    kv_pairs: RepeatWithSeparator<KVPair, CommaSeparator>,
    _ws2: Option<Whitespace>,
    "}",
);

pub fn parse(input: &str) -> JSON {
    JSON {}
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
        assert!(JSONValue::try_match(r#"5"#, 0).is_some());
        assert!(QuotedString::try_match(r#""abc""#, 0).is_some());
        assert!(KVPair::try_match(r#""abc":5,"#, 0).is_some());

        let (unit, _) = Dictionary::try_match(r#"{"abc": 5}"#, 0).unwrap();
        assert_range!(
            unit,
            r#"{"abc": 5}"#, //
            r#"^^^^^^^^^^"#,
        );

        assert_eq!(unit.kv_pairs.len(), 1);

        assert_range!(
            unit.kv_pairs.inner[0],
            r#"{"abc": 5}"#, //
            r#" ^^^^^^^^"#,
        );

        assert_range!(
            unit.kv_pairs.inner[0].key,
            r#"{"abc": 5}"#, //
            r#" ^^^^^"#,
        );

        assert_range!(
            unit.kv_pairs.inner[0].value,
            r#"{"abc": 5}"#, //
            r#"        ^"#,
        );
    }

    #[test]
    fn test_dictionary_multi_value_match() {
        let (unit, _) = Dictionary::try_match(r#"{"abc": 5, "def": "aaa"}"#, 0).unwrap();
        assert_range!(
            unit,
            r#"{"abc": 5, "def": "aaa"}"#,
            r#"^^^^^^^^^^^^^^^^^^^^^^^^"#,
        );

        assert_eq!(unit.kv_pairs.len(), 2);

        assert_range!(
            unit.kv_pairs.inner[0],
            r#"{"abc": 5, "def": "aaa"}"#,
            r#" ^^^^^^^^"#,
        );

        assert_range!(
            unit.kv_pairs.inner[1],
            r#"{"abc": 5, "def": "aaa"}"#,
            r#"           ^^^^^^^^^^^^"#,
        );

        if let JSONValue::Number(num) = &unit.kv_pairs.inner[0].value {
            assert_eq!(num.value, 5.0);
        } else {
            panic!(
                "value {:?} didn't match pattern!",
                unit.kv_pairs.inner[0].value
            );
        }

        if let JSONValue::String(s) = &unit.kv_pairs.inner[1].value {
            assert_eq!(&s.value, "aaa");
        } else {
            panic!(
                "value {:?} didn't match pattern!",
                unit.kv_pairs.inner[1].value
            );
        }
    }
}

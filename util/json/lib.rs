// A JSON parser implemented using ggen

use ggen::{Numeric, QuotedString, Whitespace};

pub struct JSON {}

ggen::one_of!(JSONValue, Number: Numeric, String: QuotedString);

ggen::sequence!(
    KVPair,
    key: QuotedString,
    _ws1: Option<Whitespace>,
    ":",
    _ws2: Option<Whitespace>,
    value: JSONValue,
    _ws3: Option<Whitespace>,
    ",",
    _ws4: Option<Whitespace>,
);

ggen::sequence!(
    Dictionary,
    "{",
    _ws1: Option<Whitespace>,
    kv_pairs: Vec<KVPair>,
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

        let (unit, _) = Dictionary::try_match(r#"{"abc": 5,}"#, 0).unwrap();
        assert_range!(
            unit,
            r#"{"abc": 5,}"#, //
            r#"^^^^^^^^^^^"#,
        );
    }
}

use ggen::{
    GrammarUnit, Identifier, Numeric, ParseError, QuotedString, RepeatWithSeparator,
    Whitespace as NewlineWhitespace,
};

ggen::sequence!(
    Module,
    _ws1: Option<NewlineWhitespace>,
    bindings: RepeatWithSeparator<Assignment, Newline>,
    value: Option<Expression>,
);

// Newlines are not considered part of whitespace in ccl
ggen::char_rule!(Whitespace, |ch: char| ch.is_whitespace() && ch != '\n');

ggen::sequence!(
    Newline,
    _ws1: Option<Whitespace>,
    "\n",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    Assignment,
    left: Identifier,
    _ws1: Option<Whitespace>,
    "=",
    _ws2: Option<Whitespace>,
    right: Expression,
);

ggen::unit!(Null, "null");

ggen::sequence!(
    Dictionary,
    "{",
    _ws1: Option<NewlineWhitespace>,
    values: RepeatWithSeparator<Assignment, Newline>,
    "}",
);

ggen::one_of!(
    CCLValue,
    Identifier: Identifier,
    Numeric: Numeric,
    String: QuotedString,
    Null: Null,
    Dictionary: Dictionary
);

ggen::one_of!(
    Expression,
    Identifier: Identifier,
    Numeric: Numeric,
    String: QuotedString
);

pub fn get_ast(content: &str) -> Result<Module, ParseError> {
    Module::try_match(content, 0).map(|(module, _, _)| module)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_range {
        ($unit:expr, $content:expr, $expected:expr,) => {
            let (start, end) = $unit.range();
            assert_eq!($expected, &$content[start..end],);
        };
    }

    #[test]
    fn test_parse_assignment() {
        let content = "a = 5.5";
        let (unit, _, _) = Identifier::try_match(content, 0).unwrap();
        assert_eq!(unit.range(), (0, 1));

        let (unit, _, _) = Whitespace::try_match("   ", 0).unwrap();
        assert_eq!(unit.range(), (0, 3));

        let unit = Assignment::try_match(content, 0).unwrap();
    }

    //#[test]
    fn test_parse_ast() {
        let content = r#"
a = "hello"
b = "world"
b
        "#;
        let unit = get_ast(content).unwrap();

        assert_eq!(unit.bindings.inner.len(), 2);
        assert_range!(unit.bindings.inner[0].left, content, "a",);
        assert!(matches!(
            unit.bindings.inner[0].right,
            Expression::String(_)
        ));
        assert_range!(unit.bindings.inner[1].left, content, "b",);
        assert_range!(unit.value, content, "b",);
    }
}

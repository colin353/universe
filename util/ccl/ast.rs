use ggen::{
    AtLeastOne, GrammarUnit, Identifier, Numeric, ParseError, QuotedString, RepeatWithSeparator,
    Whitespace as NewlineWhitespace,
};

ggen::sequence!(
    Module,
    _ws1: Option<NewlineWhitespace>,
    bindings: RepeatWithSeparator<Assignment, AtLeastOne<Newline>>,
    _ws2: Newline,
    value: Option<Expression>,
    _ws3: Option<NewlineWhitespace>,
);

// Newlines are not considered part of whitespace in ccl
ggen::char_rule!(Whitespace, |ch: char| ch.is_whitespace() && ch != '\n');

ggen::unit!(Period, ".");
type CCLIdentifier = RepeatWithSeparator<Identifier, Period>;

ggen::sequence!(
    Newline,
    _ws1: Option<Whitespace>,
    "\n",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    Assignment,
    left: CCLIdentifier,
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
    _ws2: Option<NewlineWhitespace>,
    "}",
);

ggen::one_of!(
    CCLValue,
    Identifier: CCLIdentifier,
    Numeric: Numeric,
    String: QuotedString,
    Null: Null,
    Dictionary: Dictionary
);

ggen::one_of!(Expression, Value: CCLValue);

pub fn get_ast(content: &str) -> Result<Module, ParseError> {
    Module::try_match(content, 0).map(|(module, _, _)| module)
}

pub fn get_ast_or_panic(content: &str) -> Module {
    let errors = match Module::try_match(content, 0) {
        Ok((module, took, _)) => {
            if took == content.len() {
                return module;
            }

            ParseError::new(
                String::from("unexpected extra content"),
                "module",
                took,
                took + 1,
            )
        }
        Err(e) => e,
    };

    println!("Failed to parse content!");
    println!("{}", errors.render(content));
    panic!("Can't continue!");
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

        Assignment::try_match(content, 0).unwrap();
    }

    #[test]
    fn test_parse_ast() {
        let content = r#"
a = "hello"
b = "world"
b
        "#;
        let unit = get_ast_or_panic(content);

        assert_eq!(unit.bindings.inner.len(), 2);
        assert_range!(unit.bindings.inner[0].left, content, "a",);
        assert!(matches!(
            unit.bindings.inner[0].right,
            Expression::Value(CCLValue::String(_))
        ));
        assert_range!(unit.bindings.inner[1].left, content, "b",);
        assert_range!(unit.value, content, "b",);
    }

    #[test]
    fn test_parse_dict() {
        let content = r#"
a = {
    a = "hello"
}
b = a.a
b
        "#;
        let unit = get_ast_or_panic(content);

        assert_eq!(unit.bindings.inner.len(), 2);
        assert_range!(unit.bindings.inner[0].left, content, "a",);
        assert!(matches!(
            unit.bindings.inner[0].right,
            Expression::Value(CCLValue::Dictionary(_))
        ));
        assert_range!(unit.bindings.inner[1].left, content, "b",);
        assert!(unit.value.is_some());
        assert_range!(unit.value, content, "b",);
    }
}

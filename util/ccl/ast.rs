use ggen::{
    AtLeastOne, Comment, GrammarUnit, Identifier, Numeric, ParseError, QuotedString,
    RepeatWithSeparator, Whitespace as NewlineWhitespace,
};

ggen::sequence!(
    Module,
    _ws1: Vec<WhitespaceNewlineComment>,
    imports: Vec<ImportStatement>,
    _ws2: Vec<WhitespaceNewlineComment>,
    bindings: Vec<AssignmentLine>,
    value: Option<Expression>,
    _ws3: Vec<WhitespaceNewlineComment>,
    _ws4: Option<Whitespace>,
    comment: Option<Comment>, // possible trailing comment w/ no newline
);

ggen::sequence!(
    AssignmentLine,
    assignment: Assignment,
    comments: AtLeastOne<WhitespaceNewlineComment>,
);

// Newlines are not considered part of whitespace in ccl
ggen::char_rule!(Whitespace, |ch: char| ch.is_whitespace() && ch != '\n');

ggen::unit!(Period, ".");
pub type CCLIdentifier = RepeatWithSeparator<Identifier, Period>;

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

ggen::sequence!(
    Array,
    "[",
    _ws1: Option<NewlineWhitespace>,
    values: Option<RepeatWithSeparator<Expression, CommaSeparator>>,
    _comma: Option<CommaSeparator>,
    _ws2: Option<NewlineWhitespace>,
    "]",
);

ggen::sequence!(
    CommaSeparator,
    _ws1: Option<Whitespace>,
    ",",
    _ws2: Option<NewlineWhitespace>,
);

ggen::unit!(Null, "null");
ggen::unit!(True, "true");
ggen::unit!(False, "false");
ggen::one_of!(Boolean, True: True, False: False);

ggen::sequence!(
    Dictionary,
    "{",
    _ws1: Vec<WhitespaceNewlineComment>,
    values: RepeatWithSeparator<Assignment, AtLeastOne<WhitespaceNewlineComment>>,
    _ws2: Vec<WhitespaceNewlineComment>,
    "}",
);

ggen::sequence!(
    WhitespaceNewlineComment,
    _ws1: Option<Whitespace>,
    comment: Option<Comment>,
    "\n",
    _ws2: Option<Whitespace>,
);

ggen::one_of!(
    CCLValue,
    Bool: Boolean,
    Null: Null,
    Identifier: CCLIdentifier,
    Numeric: Numeric,
    String: QuotedString,
    Dictionary: Dictionary,
    Array: Array
);

ggen::one_of!(
    ValueExpression,
    SubExpression: SubExpression,
    Value: CCLValue
);

ggen::one_of!(
    Operator,
    Addition: AdditionOperator,
    Subtraction: SubtractionOperator,
    Multiplication: MultiplicationOperator,
    Division: DivisionOperator,
    And: AndOperator,
    Or: OrOperator
);

ggen::sequence!(
    OperatorExpression,
    value: ValueExpression,
    operator: Operator,
    continuation: RepeatWithSeparator<ValueExpression, Operator>,
);

ggen::sequence!(
    AndOperator,
    _ws1: Option<Whitespace>,
    "&&",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    OrOperator,
    _ws1: Option<Whitespace>,
    "||",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    MultiplicationOperator,
    _ws1: Option<Whitespace>,
    "*",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    DivisionOperator,
    _ws1: Option<Whitespace>,
    "/",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    AdditionOperator,
    _ws1: Option<Whitespace>,
    "+",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    SubtractionOperator,
    _ws1: Option<Whitespace>,
    "-",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    SubExpression,
    "(",
    _ws1: Option<NewlineWhitespace>,
    expression: Expression,
    _ws2: Option<NewlineWhitespace>,
    ")",
);

ggen::sequence!(
    ExpansionExpression,
    identifier: CCLIdentifier,
    _ws1: Option<Whitespace>,
    target: Dictionary,
);

ggen::one_of!(
    Expression,
    OperatorExpression: OperatorExpression,
    SubExpression: SubExpression,
    ExpansionExpression: ExpansionExpression,
    Value: CCLValue
);

ggen::sequence!(
    ImportStatement,
    "import",
    _ws1: AtLeastOne<Whitespace>,
    spec: ImportSpecification,
    _ws2: AtLeastOne<Whitespace>,
    "from",
    _ws3: AtLeastOne<Whitespace>,
    from: QuotedString,
    _term: AtLeastOne<WhitespaceNewlineComment>,
);

ggen::one_of!(
    ImportSpecification,
    Multiple: MultipleBindings,
    Single: Identifier
);

ggen::sequence!(
    MultipleBindings,
    "{",
    _ws1: Option<Whitespace>,
    identifiers: RepeatWithSeparator<Identifier, CommaSeparator>,
    _ws2: Option<Whitespace>,
    "}",
);

pub fn get_ast(content: &str) -> Result<Module, ParseError> {
    let errors = match Module::try_match(content, 0) {
        Ok((module, took, _)) => {
            if took == content.len() {
                return Ok(module);
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

    Err(errors)
}

pub fn get_ast_or_panic(content: &str) -> Module {
    let errors = match get_ast(content) {
        Ok(module) => return module,
        Err(e) => e,
    };

    eprintln!("Failed to parse content!");
    eprintln!("{}", errors.render(content));
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

    macro_rules! assert_matches {
        ($expected:expr, $pattern:pat) => {
            assert!(
                matches!($expected, $pattern),
                "didn't match! expected {}, got {:?}",
                stringify!($pattern),
                $expected
            );
        };
    }

    macro_rules! assert_parse {
        ($unit:ty, $content:expr) => {
            if let Err(e) = <$unit>::try_match($content, 0) {
                println!("failed to parse:\n{}", e.render($content));
                panic!("can't continue!");
            }
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

        assert_eq!(unit.bindings.len(), 2);
        assert_range!(unit.bindings[0].assignment.left, content, "a",);
        assert!(matches!(
            unit.bindings[0].assignment.right,
            Expression::Value(_)
        ));
        assert_range!(unit.bindings[1].assignment.left, content, "b",);
        assert_range!(unit.value, content, "b",);
    }

    #[test]
    fn test_parse_dict() {
        let content = r#"
a = {
    a = "hello"
}
b = a.a
c = a + (2 + 3)
b
        "#;
        let unit = get_ast_or_panic(content);

        assert_eq!(unit.bindings.len(), 3);
        assert_range!(unit.bindings[0].assignment.left, content, "a",);
        assert!(
            matches!(unit.bindings[0].assignment.right, Expression::Value(_)),
            "didn't match! {:?}",
            unit.bindings[0].assignment.right
        );
        assert_range!(unit.bindings[1].assignment.left, content, "b",);

        assert_range!(unit.bindings[2].assignment.left, content, "c",);
        assert_range!(unit.bindings[2].assignment.right, content, "a + (2 + 3)",);
        assert_matches!(
            unit.bindings[2].assignment.right,
            Expression::OperatorExpression(_)
        );

        assert!(unit.value.is_some());
        assert_range!(unit.value, content, "b",);
    }

    #[test]
    fn test_parse_expansion() {
        let content = r#"
a = {
    a = "hello"
}
b = a {
    b = "world"
}
b
        "#;
        let unit = get_ast_or_panic(content);

        assert_eq!(unit.bindings.len(), 2);
        assert_range!(unit.bindings[1].assignment.left, content, "b",);
        assert_matches!(
            unit.bindings[1].assignment.right,
            Expression::ExpansionExpression(_)
        );

        assert!(unit.value.is_some());
        assert_range!(unit.value, content, "b",);
    }

    #[test]
    fn test_parse_subexpressions() {
        get_ast_or_panic(
            "
            a = (  2+ 3)
        ",
        );
    }

    #[test]
    fn test_parse_import() {
        get_ast_or_panic(
            r#"
            import { ABC, DEF } from "../zzz.ccl"
            import zzz from "../zzz.ccl"
       "#,
        );
    }
}

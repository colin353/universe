use ggen::GrammarUnit;

use crate::ast;

pub fn format(module: ast::Module, content: &str) -> String {
    let mut output = String::new();

    for binding in &module.bindings.values {
        format_binding(binding, content, &mut output);
        output.push('\n');
    }

    output
}

pub fn format_binding(binding: &ast::Assignment, content: &str, dest: &mut String) {
    dest.push_str(&format!(
        "{} = {}",
        format_unit(&binding.left, content),
        format_expression(&binding.right, content)
    ));
}

pub fn format_unit<'a, G: GrammarUnit>(unit: &G, content: &'a str) -> &'a str {
    let (start, end) = unit.range();
    &content[start..end]
}

pub fn format_expression(expr: &ast::Expression, content: &str) -> String {
    match expr {
        ast::Expression::SubExpression(sub) => {
            format!("({})", format_expression(&sub.expression, content))
        }
        ast::Expression::OperatorExpression(op) => format_operator_expression(op, content),
        ast::Expression::ExpansionExpression(exp) => format!(
            "{} {}",
            format_unit(&exp.identifier, &content),
            format_unit(&exp.target, &content)
        ),
        ast::Expression::Value(value) => format_unit(value.as_ref(), &content).to_string(),
    }
}

pub fn format_value_expression(expr: &ast::ValueExpression, content: &str) -> String {
    match expr {
        ast::ValueExpression::SubExpression(sub) => {
            format!("({})", format_expression(&sub.expression, content))
        }
        ast::ValueExpression::Value(value) => format_unit(value.as_ref(), &content).to_string(),
    }
}

pub fn format_operator_expression(expr: &ast::OperatorExpression, content: &str) -> String {
    let mut continuation = if expr.continuation.values.is_empty() {
        String::from("")
    } else {
        String::from(" ")
    };

    let mut values = expr.continuation.values.iter();
    let mut operators = expr.continuation.separators.iter();
    while let Some(value) = values.next() {
        continuation += &format_value_expression(value, &content);
        if let Some(op) = operators.next() {
            continuation.push_str(" ");
            continuation.push_str(format_operator(op));
            continuation.push_str(" ");
        }
    }

    format!(
        "{} {}{}",
        format_value_expression(&expr.value, &content),
        format_operator(&expr.operator),
        continuation,
    )
}

pub fn format_operator(operator: &ast::Operator) -> &str {
    match operator {
        ast::Operator::Addition(_) => "+",
        ast::Operator::Subtraction(_) => "-",
    }
}

pub fn format_dictionary(dict: &ast::Dictionary, content: &str) -> String {
    // TODO: format dictionaries somehow?
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_fmt {
        ($input:expr, $expected:expr,) => {
            let parsed = ast::get_ast_or_panic($input);
            assert_eq!(format(parsed, $input).trim(), $expected.trim());
        };
    }

    #[test]
    fn test_formatting() {
        assert_fmt!(
            "
            asdf    =  3.5
        ",
            "asdf = 3.5",
        );

        assert_fmt!(
            "
            a = (  2+3       )
        ",
            "a = (2 + 3)",
        );

        assert_fmt!(
            "
            a = (  \"hello\"+3 - 2      )
        ",
            "a = (\"hello\" + 3 - 2)",
        );
    }
}

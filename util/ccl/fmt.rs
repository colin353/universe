use ggen::GrammarUnit;

use crate::ast;

pub fn format(module: ast::Module, content: &str) -> String {
    let mut output = String::new();

    // Print any leading comments
    output.push_str(&format_comment(module._ws1.iter(), content, "", false));

    let mut separators = module.bindings.separators.iter();
    for binding in &module.bindings.values {
        format_binding(binding, content, "", &mut output);

        if let Some(sep) = separators.next() {
            output.push_str(&format_comment(sep.inner.iter(), content, "", true));
        }
    }

    output.push_str(&format_comment(
        module._ws2.inner.iter(),
        content,
        "",
        false,
    ));

    if let Some(expr) = module.value {
        output.push_str(&format_expression(&expr, content, ""));
    }

    let trailing_comment = format_comment(module._ws3.iter(), content, "", true);
    let trailing_comment = trailing_comment.trim_end_matches('\n');
    output.push_str(trailing_comment);

    if let Some(comment) = module.comment {
        if module._ws3.is_empty() {
            output.push(' ');
        }
        output.push_str(&format_unit(&comment, content));
    }
    output.push('\n');

    output
}

pub fn format_comment<'a, I: Iterator<Item = &'a ast::WhitespaceNewlineComment>>(
    comments: I,
    content: &str,
    indent: &str,
    mut directly_trailing: bool,
) -> String {
    let mut output = String::new();
    let mut bare_newline_count = 0;
    for wc in comments {
        if let Some(c) = &wc.comment {
            if directly_trailing {
                output.push(' ');
            } else {
                output.push_str(indent);
            }
            output.push_str(format_unit(c, content));
            bare_newline_count = 0;
        } else {
            bare_newline_count += 1;
        }

        if bare_newline_count <= 2 {
            output.push('\n');
        }

        directly_trailing = false;
    }
    output
}

pub fn format_binding(binding: &ast::Assignment, content: &str, indent: &str, dest: &mut String) {
    dest.push_str(&format!(
        "{} = {}",
        format_unit(&binding.left, content),
        format_expression(&binding.right, content, indent)
    ));
}

pub fn format_unit<'a, G: GrammarUnit>(unit: &G, content: &'a str) -> &'a str {
    let (start, end) = unit.range();
    &content[start..end]
}

pub fn format_expression(expr: &ast::Expression, content: &str, indent: &str) -> String {
    match expr {
        ast::Expression::SubExpression(sub) => {
            format!("({})", format_expression(&sub.expression, content, indent))
        }
        ast::Expression::OperatorExpression(op) => format_operator_expression(op, content, indent),
        ast::Expression::ExpansionExpression(exp) => format!(
            "{} {}",
            format_unit(&exp.identifier, content),
            format_dictionary(&exp.target, content, indent)
        ),
        ast::Expression::Value(value) => format_value(value.as_ref(), &content, indent),
    }
}

pub fn format_value(value: &ast::CCLValue, content: &str, indent: &str) -> String {
    match value {
        ast::CCLValue::Dictionary(dict) => format_dictionary(&dict, content, indent),
        _ => format_unit(value, content).to_string(),
    }
}

pub fn format_value_expression(expr: &ast::ValueExpression, content: &str, indent: &str) -> String {
    match expr {
        ast::ValueExpression::SubExpression(sub) => {
            format!("({})", format_expression(&sub.expression, content, indent))
        }
        ast::ValueExpression::Value(value) => format_value(value.as_ref(), &content, indent),
    }
}

pub fn format_operator_expression(
    expr: &ast::OperatorExpression,
    content: &str,
    indent: &str,
) -> String {
    let mut continuation = if expr.continuation.values.is_empty() {
        String::from("")
    } else {
        String::from(" ")
    };

    let mut values = expr.continuation.values.iter();
    let mut operators = expr.continuation.separators.iter();
    while let Some(value) = values.next() {
        continuation += &format_value_expression(value, &content, indent);
        if let Some(op) = operators.next() {
            continuation.push_str(" ");
            continuation.push_str(format_operator(op));
            continuation.push_str(" ");
        }
    }

    format!(
        "{} {}{}",
        format_value_expression(&expr.value, &content, indent),
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

pub fn format_dictionary(dict: &ast::Dictionary, content: &str, indent: &str) -> String {
    let inner_indent = format!("{}    ", indent);

    if dict.values.is_empty() {
        return String::from("{}");
    }
    let mut output = String::from("{\n");
    let leading_comment = format_comment(dict._ws1.iter(), content, &inner_indent, false);
    let leading_comment = leading_comment.trim_matches('\n');
    if leading_comment.len() > 0 {
        output.push_str(&leading_comment);
        output.push('\n');
    }

    let mut values = dict.values.values.iter();
    let mut separators = dict.values.separators.iter();
    for value in values {
        output.push_str(&inner_indent);
        format_binding(value, content, &inner_indent, &mut output);

        if let Some(separator) = separators.next() {
            output.push_str(&format_comment(
                separator.inner.iter(),
                content,
                &inner_indent,
                true,
            ));
        }
    }

    let trailing_comment = format_comment(dict._ws2.iter(), content, &inner_indent, true);
    let trailing_comment = trailing_comment.trim_matches('\n');
    if trailing_comment.len() > 0 {
        output.push_str(&trailing_comment);
    }

    output.push('\n');
    output.push_str(indent);
    output.push('}');

    output
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

    #[test]
    fn test_formatting_comments() {
        assert_fmt!(
            r#"
// leading comment
a = (2+3)
// inner comment
b = {
x = {
c = "test one two three" // this is a comment
// another one

}
}
3.5 // trailing comment"#,
            r#"
// leading comment
a = (2 + 3)
// inner comment
b = {
    x = {
        c = "test one two three" // this is a comment
        // another one
    }
}
3.5 // trailing comment"#,
        );
    }
}

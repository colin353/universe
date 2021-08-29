use crate::ast;
use crate::exec::{ExecError, Scope, ValueOrScope};
use crate::Value;

use ggen::GrammarUnit;

use std::collections::HashMap;

pub fn get_dependencies(expr: &ast::Expression) -> Vec<ast::CCLIdentifier> {
    match expr {
        ast::Expression::SubExpression(sub) => get_dependencies(&sub.expression),
        ast::Expression::OperatorExpression(opex) => Vec::new(),
        ast::Expression::ExpansionExpression(expansion) => Vec::new(),
        ast::Expression::Value(value) => match value.as_ref() {
            ast::CCLValue::Identifier(ident) => vec![ident.as_ref().clone()],
            _ => vec![],
        },
    }
}

pub fn evaluate<'a>(
    expr: &ast::Expression,
    content: &'a str,
    dependencies: &HashMap<String, ValueOrScope<'a>>,
) -> Result<ValueOrScope<'a>, ExecError> {
    println!(
        "evaluation:\n\n  {}\n\nwith:\n{:#?}",
        expr.as_str(content),
        dependencies
    );
    match expr {
        ast::Expression::SubExpression(sub) => evaluate(&sub.expression, content, dependencies),
        ast::Expression::OperatorExpression(opex) => Ok(ValueOrScope::Value(Value::Null)),
        ast::Expression::ExpansionExpression(expansion) => Ok(ValueOrScope::Value(Value::Null)),
        ast::Expression::Value(value) => match value.as_ref() {
            ast::CCLValue::Identifier(ident) => {
                let name = ident.as_str(content);
                Ok(dependencies
                    .get(name)
                    .expect("request dependency, but didn't get it!")
                    .to_owned())
            }
            ast::CCLValue::Numeric(value) => Ok(ValueOrScope::Value(Value::Number(value.value))),
            ast::CCLValue::String(value) => {
                Ok(ValueOrScope::Value(Value::String(value.value.clone())))
            }
            ast::CCLValue::Null(_) => Ok(ValueOrScope::Value(Value::Null)),
            ast::CCLValue::Dictionary(dict) => Ok(ValueOrScope::Scope(Scope::from_dictionary(
                dict.as_ref().clone(),
                content,
            ))),
        },
    }
}

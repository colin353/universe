use crate::exec::{ExecError, Scope, ValueOrScope};
use crate::{ast, Value};
use ggen::ParseError;

use ggen::GrammarUnit;

use std::borrow::Cow;
use std::collections::HashMap;

pub fn get_dependencies(expr: &ast::Expression) -> Vec<ast::CCLIdentifier> {
    match expr {
        ast::Expression::SubExpression(sub) => get_dependencies(&sub.expression),
        ast::Expression::OperatorExpression(opex) => {
            let values: Vec<_> = std::iter::once(&opex.value)
                .chain(opex.continuation.values.iter())
                .collect();
            let mut out = Vec::new();
            for value in values {
                let deps = match value {
                    ast::ValueExpression::Value(v) => {
                        get_dependencies(&ast::Expression::Value(v.clone()))
                    }
                    ast::ValueExpression::SubExpression(sub) => get_dependencies(&sub.expression),
                };
                out.extend(deps);
            }
            out
        }
        ast::Expression::ExpansionExpression(expansion) => vec![expansion.identifier.clone()],
        ast::Expression::Value(value) => match value.as_ref() {
            ast::CCLValue::Identifier(ident) => vec![ident.as_ref().clone()],
            ast::CCLValue::Array(array) => {
                let elements = match array.values.as_ref() {
                    Some(e) => &e.values,
                    None => return Vec::new(),
                };
                elements
                    .iter()
                    .map(|e| get_dependencies(&e).into_iter())
                    .flatten()
                    .collect()
            }
            _ => vec![],
        },
    }
}

pub fn evaluate<'a>(
    expr: &ast::Expression,
    content: Cow<'a, str>,
    dependencies: &HashMap<String, ValueOrScope<'a>>,
) -> Result<ValueOrScope<'a>, ExecError> {
    match expr {
        ast::Expression::SubExpression(sub) => evaluate(&sub.expression, content, dependencies),
        ast::Expression::OperatorExpression(opex) => {
            evaluate_operator_expression(&opex, content, dependencies)
        }
        ast::Expression::ExpansionExpression(expansion) => {
            evaluate_expansion(&expansion, content, dependencies)
        }
        ast::Expression::Value(value) => match value.as_ref() {
            ast::CCLValue::Identifier(ident) => {
                let name = ident.as_str(content.as_ref());
                Ok(dependencies
                    .get(name)
                    .expect("request dependency, but didn't get it!")
                    .to_owned())
            }
            ast::CCLValue::Numeric(value) => Ok(ValueOrScope::Value(Value::Number(value.value))),
            ast::CCLValue::Bool(inner) => match inner.as_ref() {
                ast::Boolean::True(_) => Ok(ValueOrScope::Value(Value::Bool(true))),
                ast::Boolean::False(_) => Ok(ValueOrScope::Value(Value::Bool(false))),
            },
            ast::CCLValue::String(value) => {
                Ok(ValueOrScope::Value(Value::String(value.value.clone())))
            }
            ast::CCLValue::Null(_) => Ok(ValueOrScope::Value(Value::Null)),
            ast::CCLValue::Dictionary(dict) => Ok(ValueOrScope::Scope(Scope::from_dictionary(
                dict.as_ref().clone(),
                content,
            ))),
            ast::CCLValue::Array(array) => {
                let mut output = Vec::new();
                let elements = match array.values.as_ref() {
                    Some(e) => &e.values,
                    None => return Ok(ValueOrScope::Value(Value::Array(Vec::new()))),
                };

                for expr in elements {
                    let inner = match evaluate(expr, content.clone(), dependencies)? {
                        ValueOrScope::Scope(_) => {
                            let (start, end) = expr.range();
                            return Err(ExecError::ArraysCannotContainDictionaries(
                                ParseError::new(
                                    String::from("dictionaries within arrays are unsupported"),
                                    "",
                                    start,
                                    end - start,
                                ),
                            ));
                        }
                        ValueOrScope::Value(value) => value,
                    };
                    output.push(inner);
                }
                Ok(ValueOrScope::Value(Value::Array(output)))
            }
        },
    }
}

#[derive(Debug)]
enum Term {
    Tree(usize),
    Value(usize),
}

#[derive(Debug)]
struct Tree<'a> {
    left: Term,
    operator: &'a ast::Operator,
    right: Term,
}

fn evaluate_operator_expression<'a>(
    expr: &ast::OperatorExpression,
    content: Cow<'a, str>,
    dependencies: &HashMap<String, ValueOrScope<'a>>,
) -> Result<ValueOrScope<'a>, ExecError> {
    let values: Vec<_> = std::iter::once(&expr.value)
        .chain(expr.continuation.values.iter())
        .collect();
    let operators: Vec<_> = std::iter::once(&expr.operator)
        .chain(expr.continuation.separators.iter())
        .collect();

    let mut value_resolution: Vec<Option<usize>> = vec![None; values.len()];
    let mut operator_resolution: Vec<Option<usize>> = vec![None; operators.len()];
    let mut tree = Vec::new();

    loop {
        // Determine the highest precedence unresolved operator
        let mut operator_to_resolve = None;
        for (idx, op) in operators.iter().enumerate() {
            if operator_resolution[idx].is_some() {
                continue;
            }

            fn operator_precedence(op: &ast::Operator) -> usize {
                match op {
                    ast::Operator::Multiplication(_) | ast::Operator::Division(_) => 5,
                    ast::Operator::Addition(_) | ast::Operator::Subtraction(_) => 4,
                    ast::Operator::And(_) => 3,
                    ast::Operator::Or(_) => 2,
                }
            }

            if let Some(other_idx) = operator_to_resolve {
                if operator_precedence(op) > operator_precedence(operators[other_idx]) {
                    operator_to_resolve = Some(idx);
                }
            } else {
                operator_to_resolve = Some(idx);
                continue;
            }
        }

        let operator_idx = match operator_to_resolve {
            Some(idx) => idx,
            None => break,
        };
        let operator = operators[operator_idx];
        let tree_idx = tree.len();

        // Mark the operator as resolved
        operator_resolution[operator_idx] = Some(tree_idx);

        let mut encompassed_terms = Vec::new();
        let left = match value_resolution[operator_idx] {
            Some(idx) => {
                encompassed_terms.push(idx);
                Term::Tree(idx)
            }
            None => {
                value_resolution[operator_idx] = Some(tree_idx);
                Term::Value(operator_idx)
            }
        };

        let right = match value_resolution[operator_idx + 1] {
            Some(idx) => {
                encompassed_terms.push(idx);
                Term::Tree(idx)
            }
            None => {
                value_resolution[operator_idx + 1] = Some(tree_idx);
                Term::Value(operator_idx + 1)
            }
        };

        // Rewrite the encompassed terms
        for (idx, _) in values.iter().enumerate() {
            if let Some(other) = value_resolution[idx] {
                if encompassed_terms.contains(&other) {
                    value_resolution[idx] = Some(tree_idx);
                }
            }
        }

        tree.push(Tree {
            left,
            right,
            operator: &operator,
        });
    }

    fn evaluate_tree<'a>(
        tree_idx: usize,
        tree: &[Tree],
        values: &[&ast::ValueExpression],
        content: Cow<'a, str>,
        dependencies: &HashMap<String, ValueOrScope<'a>>,
    ) -> Result<ValueOrScope<'a>, ExecError> {
        let left = match tree[tree_idx].left {
            Term::Tree(idx) => evaluate_tree(idx, tree, values, content.clone(), dependencies)?,
            Term::Value(idx) => match &values[idx] {
                ast::ValueExpression::SubExpression(sub) => {
                    evaluate(&sub.expression, content.clone(), dependencies)?
                }
                ast::ValueExpression::Value(value) => evaluate(
                    &ast::Expression::Value(value.clone()),
                    content.clone(),
                    dependencies,
                )?,
            },
        };

        let right = match tree[tree_idx].right {
            Term::Tree(idx) => evaluate_tree(idx, tree, values, content.clone(), dependencies)?,
            Term::Value(idx) => match &values[idx] {
                ast::ValueExpression::SubExpression(sub) => {
                    evaluate(&sub.expression, content.clone(), dependencies)?
                }
                ast::ValueExpression::Value(value) => evaluate(
                    &ast::Expression::Value(value.clone()),
                    content.clone(),
                    dependencies,
                )?,
            },
        };

        match tree[tree_idx].operator {
            ast::Operator::Addition(op) => evaluate_addition(left, right, &op),
            ast::Operator::Subtraction(op) => evaluate_subtraction(left, right, &op),
            ast::Operator::Multiplication(op) => evaluate_multiplication(left, right, &op),
            ast::Operator::Division(op) => evaluate_division(left, right, &op),
            ast::Operator::And(op) => evaluate_and(left, right, &op),
            ast::Operator::Or(op) => evaluate_or(left, right, &op),
        }
    };

    // Construct the final output tree
    evaluate_tree(
        tree.len() - 1,
        &tree,
        values.as_slice(),
        content,
        dependencies,
    )
}

fn evaluate_addition<'a>(
    left: ValueOrScope<'a>,
    right: ValueOrScope<'a>,
    operator: &ast::AdditionOperator,
) -> Result<ValueOrScope<'a>, ExecError> {
    let left = match left {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `+` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    let right = match right {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `+` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    match (left, right) {
        (Value::Number(a), Value::Number(b)) => Ok(ValueOrScope::Value(Value::Number(a + b))),
        (Value::String(a), Value::String(b)) => Ok(ValueOrScope::Value(Value::String(a + &b))),
        (l, r) => {
            let (start, end) = operator.range();
            Err(ExecError::OperatorWithInvalidType(ParseError::new(
                format!(
                    "unable to use `+` operator on {} (left) and {} (right)",
                    l.type_name(),
                    r.type_name(),
                ),
                "",
                start,
                end,
            )))
        }
    }
}

fn evaluate_subtraction<'a>(
    left: ValueOrScope<'a>,
    right: ValueOrScope<'a>,
    operator: &ast::SubtractionOperator,
) -> Result<ValueOrScope<'a>, ExecError> {
    let left = match left {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `-` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    let right = match right {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `-` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    match (left, right) {
        (Value::Number(a), Value::Number(b)) => Ok(ValueOrScope::Value(Value::Number(a - b))),
        (l, r) => {
            let (start, end) = operator.range();
            Err(ExecError::OperatorWithInvalidType(ParseError::new(
                format!(
                    "unable to use `-` operator on {} (left) and {} (right)",
                    l.type_name(),
                    r.type_name(),
                ),
                "",
                start,
                end,
            )))
        }
    }
}

fn evaluate_multiplication<'a>(
    left: ValueOrScope<'a>,
    right: ValueOrScope<'a>,
    operator: &ast::MultiplicationOperator,
) -> Result<ValueOrScope<'a>, ExecError> {
    let left = match left {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `*` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    let right = match right {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `*` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    match (left, right) {
        (Value::Number(a), Value::Number(b)) => Ok(ValueOrScope::Value(Value::Number(a * b))),
        (l, r) => {
            let (start, end) = operator.range();
            Err(ExecError::OperatorWithInvalidType(ParseError::new(
                format!(
                    "unable to use `*` operator on {} (left) and {} (right)",
                    l.type_name(),
                    r.type_name(),
                ),
                "",
                start,
                end,
            )))
        }
    }
}

fn evaluate_division<'a>(
    left: ValueOrScope<'a>,
    right: ValueOrScope<'a>,
    operator: &ast::DivisionOperator,
) -> Result<ValueOrScope<'a>, ExecError> {
    let left = match left {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `/` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    let right = match right {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `/` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    match (left, right) {
        (Value::Number(a), Value::Number(b)) => {
            if b == 0.0 {
                let (start, end) = operator.range();
                return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                    String::from("division by zero!"),
                    "",
                    start,
                    end,
                )));
            }
            Ok(ValueOrScope::Value(Value::Number(a / b)))
        }
        (l, r) => {
            let (start, end) = operator.range();
            Err(ExecError::OperatorWithInvalidType(ParseError::new(
                format!(
                    "unable to use `/` operator on {} (left) and {} (right)",
                    l.type_name(),
                    r.type_name(),
                ),
                "",
                start,
                end,
            )))
        }
    }
}

fn evaluate_and<'a>(
    left: ValueOrScope<'a>,
    right: ValueOrScope<'a>,
    operator: &ast::AndOperator,
) -> Result<ValueOrScope<'a>, ExecError> {
    let left = match left {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `&&` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    let right = match right {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `&&` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    match (left, right) {
        (Value::Bool(a), Value::Bool(b)) => Ok(ValueOrScope::Value(Value::Bool(a && b))),
        (l, r) => {
            let (start, end) = operator.range();
            Err(ExecError::OperatorWithInvalidType(ParseError::new(
                format!(
                    "unable to use `&&` operator on {} (left) and {} (right)",
                    l.type_name(),
                    r.type_name(),
                ),
                "",
                start,
                end,
            )))
        }
    }
}

fn evaluate_or<'a>(
    left: ValueOrScope<'a>,
    right: ValueOrScope<'a>,
    operator: &ast::OrOperator,
) -> Result<ValueOrScope<'a>, ExecError> {
    let left = match left {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `&&` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    let right = match right {
        ValueOrScope::Value(v) => v,
        ValueOrScope::Scope(_) => {
            let (start, end) = operator.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                String::from("unable to use `||` operator on a dictionary"),
                "",
                start,
                end,
            )));
        }
    };

    fn as_bool(value: &Value) -> Option<bool> {
        match value {
            Value::Bool(x) => Some(*x),
            Value::Null => Some(false),
            Value::String(s) => Some(!s.is_empty()),
            Value::Number(n) => Some(*n != 0.0),
            _ => None,
        }
    }

    let left_as_bool = as_bool(&left);
    let right_as_bool = as_bool(&right);

    match (left_as_bool, right_as_bool) {
        (Some(true), Some(_)) => Ok(ValueOrScope::Value(left)),
        (Some(_), Some(true)) => Ok(ValueOrScope::Value(right)),
        (Some(false), Some(false)) => Ok(ValueOrScope::Value(right)),
        (None, _) | (_, None) => {
            let (start, end) = operator.range();
            Err(ExecError::OperatorWithInvalidType(ParseError::new(
                format!(
                    "unable to use `||` operator on {} (left) and {} (right)",
                    left.type_name(),
                    right.type_name()
                ),
                "",
                start,
                end,
            )))
        }
    }
}

fn evaluate_expansion<'a>(
    expansion: &ast::ExpansionExpression,
    content: Cow<'a, str>,
    dependencies: &HashMap<String, ValueOrScope<'a>>,
) -> Result<ValueOrScope<'a>, ExecError> {
    let name = expansion.identifier.as_str(content.as_ref());
    let original = match dependencies
        .get(name)
        .expect("requested dependency, but didn't get it!")
        .to_owned()
    {
        ValueOrScope::Scope(s) => s,
        ValueOrScope::Value(v) => {
            let (start, end) = expansion.identifier.range();
            return Err(ExecError::OperatorWithInvalidType(ParseError::new(
                format!("unable to expand {}", v.type_name()),
                "",
                start,
                end,
            )));
        }
    };

    let scope = original.duplicate();
    let override_scope = Scope::from_dictionary(expansion.target.clone(), content);
    for deep_override in override_scope.inner.lock().unwrap().deep_overrides.iter() {
        scope.add_deep_overrides(deep_override.0.to_owned(), deep_override.1);
    }
    scope.add_override(override_scope);

    Ok(ValueOrScope::Scope(scope))
}

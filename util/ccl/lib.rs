mod ast;
mod eval;
mod exec;
mod fmt;

pub use ast::{get_ast, get_ast_or_panic};
pub use exec::exec;
pub use fmt::format;

#[cfg(test)]
mod eval_tests;

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Dictionary(Dictionary),
}

impl Value {
    fn type_name(&self) -> &str {
        match self {
            Value::Number(_) => "a number",
            Value::String(_) => "a string",
            Value::Dictionary(_) => "a dictionary",
            Value::Null => "null",
            Value::Bool(_) => "a bool",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Dictionary {
    kv_pairs: Vec<(String, Value)>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            kv_pairs: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: Value) {
        self.kv_pairs.push((key, value));
    }
}

pub fn exec_or_panic(content: &str, specifier: &str) -> Value {
    let ast = get_ast_or_panic(content);
    match exec(ast, content, specifier) {
        Ok(x) => x,
        Err(e) => {
            panic!("failed to evaluate:\n{}", e.render(content))
        }
    }
}

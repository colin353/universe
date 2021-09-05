mod ast;
mod eval;
mod exec;
mod fmt;

pub use ast::{get_ast, get_ast_or_panic};
pub use exec::exec;
pub use fmt::format;

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

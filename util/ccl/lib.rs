mod ast;
mod eval;
mod exec;
mod fmt;
mod import_resolver;

pub use ast::{get_ast, get_ast_or_panic, Module};
pub use exec::{exec, exec_with_import_resolvers, ExecError};
pub use fmt::format;
pub use import_resolver::{FakeImportResolver, FilesystemImportResolver, ImportResolver};

#[cfg(test)]
mod eval_tests;

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Dictionary(Dictionary),
    Array(Vec<Value>),
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Value::Number(_) => "a number",
            Value::String(_) => "a string",
            Value::Dictionary(_) => "a dictionary",
            Value::Null => "null",
            Value::Bool(_) => "a bool",
            Value::Array(_) => "an array",
        }
    }

    pub fn strs(&self) -> Result<Vec<&str>, String> {
        match self {
            Value::String(s) => Ok(vec![&s]),
            Value::Array(a) => {
                let mut output = Vec::new();
                for element in a {
                    match element {
                        Value::String(s) => output.push(s.as_str()),
                        x => {
                            return Err(format!(
                                "array must contain only strings, got {}",
                                x.type_name()
                            ))
                        }
                    }
                }
                Ok(output)
            }
            x => {
                return Err(format!(
                    "expected string or an array of strings, got {}",
                    x.type_name()
                ))
            }
        }
    }
}

pub struct AST {
    content: String,
    module: Module,
}

impl AST {
    pub fn from_string(content: String) -> Result<Self, ggen::ParseError> {
        let module = get_ast(&content)?;
        Ok(Self { content, module })
    }

    pub fn get(&self, specifier: &str) -> Result<Value, ExecError> {
        // TODO: remove the clone here, and make exec work on an &'a Module
        exec(self.module.clone(), &self.content, specifier)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Dictionary {
    pub kv_pairs: Vec<(String, Value)>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            kv_pairs: Vec::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        for (k, v) in &self.kv_pairs {
            if k == key {
                return Some(v);
            }
        }
        None
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

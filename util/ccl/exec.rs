use crate::ast;
use crate::eval;
use crate::{Dictionary, Value};

use ggen::{GrammarUnit, ParseError};

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum ExecError {
    CannotResolveSymbol(ParseError),
    OperatorWithInvalidType(ParseError),
}

impl ExecError {
    pub fn render(&self, content: &str) -> String {
        match self {
            Self::CannotResolveSymbol(e) | Self::OperatorWithInvalidType(e) => e.render(content),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueOrScope<'a> {
    Value(Value),
    Scope(Scope<'a>),
}

#[derive(Debug, Clone)]
pub struct Scope<'a> {
    inner: Arc<Mutex<ScopeInner<'a>>>,
}

#[derive(Debug)]
struct ScopeInner<'a> {
    in_progress_identifiers: HashSet<String>,
    resolved_identifiers: HashMap<String, ValueOrScope<'a>>,
    unresolved_identifiers: HashMap<String, ast::Expression>,
    scopes: HashMap<String, Scope<'a>>,
    default_value: Option<ast::Expression>,
    content: &'a str,
    parent_scope: Option<Scope<'a>>,
    overrides: Vec<Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn empty(content: &'a str) -> Self {
        let inner = ScopeInner {
            in_progress_identifiers: HashSet::new(),
            resolved_identifiers: HashMap::new(),
            unresolved_identifiers: HashMap::new(),
            scopes: HashMap::new(),
            parent_scope: None,
            overrides: Vec::new(),
            default_value: None,
            content,
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn from_module(module: ast::Module, content: &'a str) -> Self {
        let mut out = Self::empty(content);
        for assignment in module.bindings.values {
            out.inner.lock().unwrap().unresolved_identifiers.insert(
                assignment.left.as_str(content).to_string(),
                assignment.right,
            );
        }

        if let Some(value) = module.value {
            out.inner
                .lock()
                .unwrap()
                .unresolved_identifiers
                .insert(String::new(), value);
        }
        out
    }

    pub fn from_dictionary(dict: ast::Dictionary, content: &'a str) -> Self {
        let mut out = Self::empty(content);
        for assignment in dict.values.values {
            out.inner.lock().unwrap().unresolved_identifiers.insert(
                assignment.left.as_str(content).to_string(),
                assignment.right,
            );
        }
        out
    }

    pub fn resolve_scope(&self, ident: &str) -> Option<Scope<'a>> {
        if let Some(s) = self.inner.lock().unwrap().scopes.get(ident) {
            return Some(s.clone());
        }
        None
    }

    pub fn keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        let overrides: Vec<Scope<'a>> = self
            .inner
            .lock()
            .unwrap()
            .overrides
            .iter()
            .map(|s| s.to_owned())
            .collect();
        for or in overrides {
            out.append(&mut or.keys());
        }

        for (k, _) in self.inner.lock().unwrap().unresolved_identifiers.iter() {
            out.push(k.to_string());
        }
        out
    }

    pub fn resolve(&self, specifier: &str, offset: usize) -> Result<Value, ExecError> {
        let out = self
            .partially_resolve(specifier, offset)
            .map(|vos| match vos {
                ValueOrScope::Value(v) => Ok(v),
                ValueOrScope::Scope(s) => {
                    let mut out = Dictionary::new();
                    for key in s.keys() {
                        let value = match s.resolve(&key, 0) {
                            Ok(v) => v,
                            Err(e) => return Err(e),
                        };
                        out.insert(key, value);
                    }

                    Ok(Value::Dictionary(out))
                }
            });
        match out {
            Ok(r) => r,
            Err(e) => Err(e),
        }
    }

    pub fn partially_resolve(
        &self,
        specifier: &str,
        offset: usize,
    ) -> Result<ValueOrScope<'a>, ExecError> {
        // If the specifier refers to another scope, resolve there first
        if let Some(idx) = specifier.find('.') {
            let prefix = &specifier[..idx];
            let suffix = &specifier[idx + 1..];

            let s = match self.resolve_scope(prefix) {
                Some(s) => s,
                None => {
                    return Err(ExecError::CannotResolveSymbol(ParseError::new(
                        format!("unable to resolve `{}`", prefix),
                        "",
                        offset,
                        offset + idx,
                    )))
                }
            };
            return s.partially_resolve(suffix, offset + idx);
        }

        // We are about to try and resolve a particular identifier. Mark it as in progress
        if !self
            .inner
            .lock()
            .unwrap()
            .in_progress_identifiers
            .insert(specifier.to_string())
        {
            return Err(ExecError::CannotResolveSymbol(ParseError::new(
                format!("circular dependency when resolving `{}`", specifier),
                "",
                offset,
                offset + specifier.len(),
            )));
        }

        // Specifier refers to a symbol in this scope. Symbol resolution order:
        // 1. Try to resolve the symbol in the override scopes
        // 2. Try to resolve the symbol in the scope itself
        // 3. If the thing is in an expression, try to resolve in a parent
        let overrides: Vec<Scope<'a>> = self
            .inner
            .lock()
            .unwrap()
            .overrides
            .iter()
            .map(|s| s.to_owned())
            .collect();
        for scope in overrides {
            let result = scope.partially_resolve(specifier, offset);
            if let Err(ExecError::CannotResolveSymbol(_)) = scope.resolve(specifier, offset) {
                continue;
            }

            // Done resolving, unlock in progress identifiers
            self.inner
                .lock()
                .unwrap()
                .in_progress_identifiers
                .remove(specifier);

            return result;
        }

        self.inner
            .lock()
            .unwrap()
            .in_progress_identifiers
            .remove(specifier);

        // Check if the identifier has already been resolved to a basic type
        if let Some(value) = self
            .inner
            .lock()
            .unwrap()
            .resolved_identifiers
            .get(specifier)
        {
            return Ok(value.clone());
        }

        let expression = match self
            .inner
            .lock()
            .unwrap()
            .unresolved_identifiers
            .get(specifier)
        {
            Some(expr) => expr.clone(),
            None => {
                return Err(ExecError::CannotResolveSymbol(ParseError::new(
                    format!("unable to resolve identifier `{}`", specifier),
                    "",
                    offset,
                    offset + specifier.len(),
                )));
            }
        };

        let content: &str = self.inner.lock().unwrap().content.clone();
        let deps = eval::get_dependencies(&expression);
        let mut resolved_dependencies = HashMap::new();

        // We will recurse and try to partially resolve all dependencies of the expression,
        // so mark the current identifier as being resolved
        self.inner
            .lock()
            .unwrap()
            .in_progress_identifiers
            .insert(specifier.to_string());

        for d in deps {
            let name = d.as_str(content);
            let (start, _) = d.range();
            let resolved = self.partially_resolve(name, start)?;
            resolved_dependencies.insert(name.to_string(), resolved.clone());
            self.inner
                .lock()
                .unwrap()
                .resolved_identifiers
                .insert(name.to_string(), resolved);
        }

        self.inner
            .lock()
            .unwrap()
            .in_progress_identifiers
            .remove(specifier);

        let out = eval::evaluate(&expression, content, &resolved_dependencies);
        println!("resolved to {:#?}", out);
        out
    }
}

pub fn exec(module: ast::Module, content: &str, specifier: &str) -> Result<Value, ExecError> {
    let root = Scope::from_module(module, content);
    root.resolve(specifier, 0)
}

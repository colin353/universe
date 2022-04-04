use crate::ast;
use crate::exec::ExecError;

pub struct ImportResolution {
    pub module: ast::Module,
    pub content: String,
    pub context: Option<String>,
}

pub trait ImportResolver: std::fmt::Debug {
    fn resolve_import(
        &self,
        name: &str,
        context: Option<&str>,
    ) -> Result<ImportResolution, ExecError>;
}

#[derive(Debug)]
pub struct FakeImportResolver {
    imports: std::collections::HashMap<String, String>,
}
impl FakeImportResolver {
    pub fn new(data: Vec<(String, String)>) -> Self {
        Self {
            imports: data.into_iter().collect(),
        }
    }
}

impl ImportResolver for FakeImportResolver {
    fn resolve_import(&self, name: &str, _: Option<&str>) -> Result<ImportResolution, ExecError> {
        if let Some(c) = self.imports.get(name) {
            let module = match ast::get_ast(c) {
                Ok(a) => a,
                Err(e) => {
                    return Err(ExecError::ImportParsingError(e));
                }
            };

            return Ok(ImportResolution {
                module,
                content: c.to_string(),
                context: None,
            });
        }

        Err(ExecError::ImportResolutionError(format!(
            "unable to resolve import {:?}",
            name
        )))
    }
}

#[derive(Debug)]
pub struct FilesystemImportResolver {}
impl ImportResolver for FilesystemImportResolver {
    fn resolve_import(
        &self,
        name: &str,
        context: Option<&str>,
    ) -> Result<ImportResolution, ExecError> {
        let mut path = std::path::Path::new(name);
        let mut pb = std::path::PathBuf::new();
        if !path.is_absolute() {
            if let Some(ctx) = context {
                pb = std::path::Path::new(ctx).join(path);
                path = &pb;
            }
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Err(ExecError::ImportResolutionError(format!(
                    "unable to open import at {:?}",
                    path
                )));
            }
        };

        let module = match ast::get_ast(&content) {
            Ok(a) => a,
            Err(e) => {
                return Err(ExecError::ImportParsingError(e));
            }
        };

        let context = match path
            .canonicalize()
            .map(|c| c.to_str().map(|c| c.to_string()))
        {
            Ok(Some(p)) => p,
            _ => {
                return Err(ExecError::ImportResolutionError(format!(
                    "failed to canonicalize import path {:?}",
                    path
                )));
            }
        };

        Ok(ImportResolution {
            module,
            content,
            context: Some(context),
        })
    }
}

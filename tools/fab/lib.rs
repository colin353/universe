use std::collections::{HashMap, HashSet};
use std::hash::Hash;

mod config;
mod environment;
mod fs;
mod resolver;

pub use environment::BuildEnvironment;
pub use fs::FilesystemResolver;
pub use resolver::Resolver;

pub type BuildHash = u64;

#[derive(Debug, Clone)]
pub struct BuildResult {
    build_hash: BuildHash,
    outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Target {
    identifier: TargetIdentifier,
    operation: Option<TargetIdentifier>,
    dependencies: HashSet<TargetIdentifier>,
    files: HashSet<String>,
    resolved: bool,
    built_dependencies: usize,
    hash: Option<BuildHash>,
    result: Option<BuildResult>,
    variables: HashMap<String, String>,
}

impl Target {
    pub fn new(identifier: TargetIdentifier) -> Self {
        Self {
            identifier,
            operation: None,
            dependencies: HashSet::new(),
            files: HashSet::new(),
            resolved: false,
            hash: None,
            result: None,
            built_dependencies: 0,
            variables: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub fn for_test(identifier: &str, deps: &[&str], files: &[&str]) -> Self {
        Self {
            identifier: TargetIdentifier::from_str(identifier),
            operation: None,
            dependencies: deps.iter().map(|d| TargetIdentifier::from_str(d)).collect(),
            files: files.into_iter().map(|s| s.to_string()).collect(),
            resolved: false,
            hash: None,
            result: None,
            built_dependencies: 0,
            variables: HashMap::new(),
        }
    }

    pub fn hash<R: resolver::Resolver, H: std::hash::Hasher>(&self, resolver: &R, hash: &mut H) {
        self.identifier.hash(hash);
        self.variables.iter().collect::<Vec<_>>().hash(hash);
        self.operation.hash(hash);
    }

    pub fn dependencies(&self) -> Vec<TargetIdentifier> {
        let mut output = self.dependencies.clone();
        if let Some(o) = &self.operation {
            output.insert(o.to_owned());
        }
        output.into_iter().collect()
    }

    pub fn build_dir(&self, root_dir: &std::path::Path) -> std::path::PathBuf {
        root_dir.join("build").join(format!(
            "{:016x}",
            self.hash
                .expect("build_hash must be resolved to get build dir")
        ))
    }

    pub fn tmp_out_dir(&self, root_dir: &std::path::Path) -> std::path::PathBuf {
        root_dir.join("tmp").join("out").join(format!(
            "{:016x}",
            self.hash
                .expect("build_hash must be resolved to get out dir")
        ))
    }

    pub fn out_dir(&self, root_dir: &std::path::Path) -> std::path::PathBuf {
        root_dir.join("out").join(format!(
            "{:016x}",
            self.hash
                .expect("build_hash must be resolved to get out dir")
        ))
    }

    pub fn op_dir(&self, root_dir: &std::path::Path) -> std::path::PathBuf {
        root_dir.join("ops").join(format!(
            "{:016x}",
            self.hash
                .expect("build_hash must be resolved to get op dir")
        ))
    }

    pub fn is_operation(&self) -> bool {
        self.operation.is_none()
    }

    pub fn op_script(&self, root_dir: &std::path::Path) -> std::path::PathBuf {
        assert!(self.is_operation());
        self.op_dir(root_dir).join("src").join(
            self.files
                .iter()
                .next()
                .expect("operation must contain an operation script!"),
        )
    }

    pub fn fully_qualified_name(&self) -> String {
        self.identifier.fully_qualified_name()
    }

    // TODO: implement ophash
    pub fn operation_hash(&self) -> BuildHash {
        0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TargetIdentifier {
    origin: String,
    name: String,
    path: String,
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new<T: Into<String>>(message: T) -> Self {
        Error {
            message: message.into(),
        }
    }

    pub fn from_errors(input: &[Error]) -> Self {
        Error {
            message: input
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("\n\n"),
        }
    }
}

impl TargetIdentifier {
    pub fn from_str_relative(identifier: &str, parent: &TargetIdentifier) -> Self {
        if identifier.starts_with(":") {
            let mut out = parent.clone();
            out.name = identifier[1..].to_string();
            return out;
        }

        Self::from_str(identifier)
    }

    pub fn empty() -> Self {
        Self {
            origin: String::new(),
            path: String::new(),
            name: String::new(),
        }
    }

    pub fn from_str(identifier: &str) -> Self {
        let mut origin = "";
        let target_name;
        let path;

        let components: Vec<_> = identifier.split("//").collect();
        let mut path_target = components[0];
        if components.len() > 1 {
            path_target = components[1];
            origin = components[0];
        }

        let components: Vec<_> = path_target.split(":").collect();
        if components.len() == 1 {
            // Implicit target name, use last dirname
            target_name = path_target
                .rsplit("/")
                .next()
                .expect("split always yields at least one part");
            path = path_target;
        } else {
            target_name = components[1];
            path = components[0];
        }

        TargetIdentifier {
            origin: origin.to_string(),
            name: target_name.to_string(),
            path: path.to_string(),
        }
    }

    pub fn build_file(&self) -> String {
        if self.path.is_empty() {
            return String::from("BUILD.ccl");
        }
        format!("{}/BUILD.ccl", self.path)
    }

    pub fn fully_qualified_name(&self) -> String {
        format!("{}//{}:{}", self.origin, self.path, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_parsing() {
        let ident = TargetIdentifier::from_str("github.com/crazy/banana@ref109//utils/parser");
        assert_eq!(
            &ident.fully_qualified_name(),
            "github.com/crazy/banana@ref109//utils/parser:parser"
        );

        let ident = TargetIdentifier::from_str("//utils/parser:parser_test");
        assert_eq!(&ident.fully_qualified_name(), "//utils/parser:parser_test");
    }

    #[test]
    fn test_relative_target_parsing() {
        let ident = TargetIdentifier::from_str("//utils/parser");
        let relative_ident = TargetIdentifier::from_str_relative(":parser_test", &ident);
        assert_eq!(
            &relative_ident.fully_qualified_name(),
            "//utils/parser:parser_test"
        );
    }
}

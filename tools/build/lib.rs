mod exec;
mod file_resolver;
mod target_resolver;

use std::collections::HashSet;

pub use file_resolver::FileResolver;
pub use target_resolver::TargetResolver;

pub type BuildHash = u64;

#[derive(Clone)]
pub struct BuildResult {
    build_hash: BuildHash,
    outputs: Vec<String>,
}

#[derive(Clone)]
pub struct Target {
    identifier: TargetIdentifier,
    operation: build_grpc_rust::Operation,
    dependencies: HashSet<TargetIdentifier>,
    files: HashSet<String>,
    resolving: bool,
    hash: Option<BuildHash>,
    result: Option<BuildResult>,
}

impl Target {
    #[cfg(test)]
    pub fn for_test(identifier: &str, deps: &[&str], files: &[&str]) -> Self {
        Self {
            identifier: TargetIdentifier::from_str(identifier),
            operation: build_grpc_rust::Operation::new(),
            dependencies: deps.iter().map(|d| TargetIdentifier::from_str(d)).collect(),
            files: files.into_iter().map(|s| s.to_string()).collect(),
            resolving: false,
            hash: None,
            result: None,
        }
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
    message: String,
}

impl Error {
    pub fn new<T: Into<String>>(message: T) -> Self {
        Error {
            message: message.into(),
        }
    }
}

impl TargetIdentifier {
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
}

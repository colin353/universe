use std::str::FromStr;

use crate::{Error, Target, TargetIdentifier};

pub fn convert_path(path: &str, relative_to: &std::path::Path) -> String {
    relative_to
        .join(path)
        .into_os_string()
        .into_string()
        .unwrap()
}

pub fn convert_to_target(ast: &ccl::AST, identifier: &TargetIdentifier) -> Result<Target, Error> {
    let config = match ast.get(&identifier.name) {
        Ok(ccl::Value::Dictionary(c)) => c,
        Ok(_) => {
            return Err(Error::new(format!(
                "build targets must be type dictionary (in {})",
                identifier.fully_qualified_name(),
            )))
        }
        Err(e) => {
            return Err(Error::new(format!(
                "couldn't find target {}: {:?}",
                identifier.fully_qualified_name(),
                e
            )))
        }
    };

    let relative_to = std::path::PathBuf::from_str(&identifier.path).unwrap();

    let mut target = Target::new(identifier.clone());
    for (k, v) in &config.kv_pairs {
        match k.as_str() {
            "deps" => match v.strs() {
                Ok(strs) => {
                    for value in strs {
                        target
                            .dependencies
                            .insert(TargetIdentifier::from_str_relative(value, identifier));
                    }
                }
                Err(e) => {
                    return Err(Error::new(format!(
                        "unable to parse deps for {:?}: {}",
                        identifier, e
                    )));
                }
            },
            "srcs" => match v.strs() {
                Ok(strs) => {
                    for value in strs {
                        target.files.insert(convert_path(value, &relative_to));
                    }
                }
                Err(e) => {
                    return Err(Error::new(format!(
                        "unable to parse deps for {:?}: {}",
                        identifier, e
                    )));
                }
            },
            "vars" => match v {
                ccl::Value::Dictionary(dict) => {
                    for (k, v) in &dict.kv_pairs {
                        match v {
                            ccl::Value::String(s) => {
                                target.variables.insert(k.to_string(), s.to_string());
                            }
                            x => {
                                return Err(Error::new(format!(
                                    "in {}, unable to parse vars: all vars must be strings, got {}",
                                    identifier.fully_qualified_name(),
                                    x.type_name(),
                                )));
                            }
                        }
                    }
                }
                x => {
                    return Err(Error::new(format!(
                        "in {}, unable to parse vars: expected dictionary, got {}",
                        identifier.fully_qualified_name(),
                        x.type_name(),
                    )));
                }
            },
            "operation" => match v {
                ccl::Value::String(s) => {
                    if !s.is_empty() {
                        target.operation = Some(TargetIdentifier::from_str_relative(s, identifier))
                    }
                }
                x => {
                    return Err(Error::new(format!(
                        "in {}, operation must be a string, got {}",
                        identifier.fully_qualified_name(),
                        x.type_name(),
                    )));
                }
            },
            _ => continue,
        }
    }

    return Ok(target);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_target() {
        let content = String::from(
            r#"
                rust_library_operation = {
                    srcs = "script.sh"
                }

                rust_library = {
                    operation = ":rust_library_operation"
                }
                
                my_target = rust_library {
                    srcs = ["goomba.txt"]
                    deps = ["//goomba:goomba_client"]
                }
        "#,
        );
        let ast = ccl::AST::from_string(content).unwrap();
        let result =
            convert_to_target(&ast, &TargetIdentifier::from_str("//util/test:my_target")).unwrap();
        assert_eq!(&result.identifier.name, "my_target");
        assert_eq!(
            result.files.iter().next().unwrap().as_str(),
            "util/test/goomba.txt"
        );
    }
}

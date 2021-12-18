use crate::{Error, Target, TargetIdentifier};

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

    let mut target = Target::new(identifier.clone());
    for (k, v) in &config.kv_pairs {
        match k.as_str() {
            "deps" => match v.strs() {
                Ok(strs) => {
                    for value in strs {
                        target
                            .dependencies
                            .insert(TargetIdentifier::from_str(value));
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
                        target.files.insert(value.to_string());
                    }
                }
                Err(e) => {
                    return Err(Error::new(format!(
                        "unable to parse deps for {:?}: {}",
                        identifier, e
                    )));
                }
            },
            "operation" => match v {
                ccl::Value::String(s) => target.operation = s.to_string(),
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
        let result = convert_to_target(&ast, &TargetIdentifier::from_str(":my_target")).unwrap();
        assert_eq!(&result.identifier.name, "my_target");
    }
}

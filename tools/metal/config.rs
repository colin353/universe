#[derive(Debug)]
enum MetalConfigError {
    CCLParseError(ggen::ParseError),
    CCLExecError(ccl::ExecError),
    ConversionError(String),
}

fn read_config(input: &str) -> Result<metal_grpc_rust::Configuration, MetalConfigError> {
    let ast = match ccl::get_ast(input) {
        Ok(a) => a,
        Err(e) => return Err(MetalConfigError::CCLParseError(e)),
    };

    let value = match ccl::exec(ast, input, "") {
        Ok(v) => v,
        Err(e) => return Err(MetalConfigError::CCLExecError(e)),
    };

    let dict = match value {
        ccl::Value::Dictionary(dict) => dict,
        v => {
            return Err(MetalConfigError::ConversionError(format!(
                "top level config must be a dictionary, got {}",
                v.type_name()
            )))
        }
    };

    let mut out = metal_grpc_rust::Configuration::new();
    extract_config("", &dict, &mut out)?;
    Ok(out)
}

fn join_prefix(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() {
        return suffix.to_string();
    }

    format!("{}.{}", prefix, suffix)
}

fn extract_config(
    prefix: &str,
    dict: &ccl::Dictionary,
    out: &mut metal_grpc_rust::Configuration,
) -> Result<(), MetalConfigError> {
    if let Some(ccl::Value::String(ty)) = dict.get("_metal_type") {
        return match ty.as_str() {
            "task" => extract_task(prefix, dict, out),
            v => Err(MetalConfigError::ConversionError(format!(
                "unrecognized _metal_type {:?}",
                v
            ))),
        };
    }

    for (k, v) in &dict.kv_pairs {
        if let ccl::Value::Dictionary(dict) = v {
            extract_config(&join_prefix(prefix, k.as_str()), dict, out)?;
        }
    }

    Ok(())
}

fn extract_task(
    prefix: &str,
    dict: &ccl::Dictionary,
    out: &mut metal_grpc_rust::Configuration,
) -> Result<(), MetalConfigError> {
    let mut task = metal_grpc_rust::Task::new();
    task.set_name(prefix.to_string());
    for (k, v) in &dict.kv_pairs {
        match k.as_str() {
            "binary" => match v {
                ccl::Value::Dictionary(bin) => task.set_binary(extract_binary(bin)?),
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "task's binary field must be a dictionary, got {:?}",
                        v.type_name()
                    )))
                }
            },
            _ => continue,
        }
    }

    // Validate task
    if task.get_binary().get_url().is_empty() {
        return Err(MetalConfigError::ConversionError(String::from(
            "task must contain a binary!",
        )));
    }

    out.mut_tasks().push(task);
    Ok(())
}

fn extract_binary(dict: &ccl::Dictionary) -> Result<metal_grpc_rust::Binary, MetalConfigError> {
    let mut binary = metal_grpc_rust::Binary::new();
    for (k, v) in &dict.kv_pairs {
        match k.as_str() {
            "url" => match v {
                ccl::Value::String(url) => binary.set_url(url.clone()),
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "binary's url field must be a string, got {:?}",
                        v.type_name()
                    )))
                }
            },
            _ => continue,
        }
    }

    // Validate the binary
    if binary.get_url().is_empty() {
        return Err(MetalConfigError::ConversionError(String::from(
            "binary must contain a url field!",
        )));
    }

    Ok(binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_config() {
        let result = read_config(
            r#"
namespace = {
    server = {
        _metal_type = "task"
        binary = {
            url = "http://test.com/server.exe"
        }
    }
}

namespace
"#,
        );
        if let Err(e) = &result {
            println!("got error: {:?}", e);
        }
        let r = result.unwrap();
        assert_eq!(r.get_tasks().len(), 1);
        let t = &r.get_tasks()[0];
        assert_eq!(t.get_name(), "server");
        assert_eq!(t.get_binary().get_url(), "http://test.com/server.exe");
    }
}

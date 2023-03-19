use metal_bus::{ArgKind, ArgValue, RestartMode};

use std::sync::Arc;

static METAL_CCL_IMPORT: &str = include_str!("metal.ccl");

#[derive(Debug)]
pub enum MetalConfigError {
    CCLParseError(ggen::ParseError),
    CCLExecError(ccl::ExecError),
    ConversionError(String),
}

pub fn read_config(input: &str) -> Result<metal_bus::Configuration, MetalConfigError> {
    let ast = match ccl::get_ast(input) {
        Ok(a) => a,
        Err(e) => return Err(MetalConfigError::CCLParseError(e)),
    };

    let mut static_resolver = ccl::StaticImportResolver::new();
    static_resolver.add_import("metal", METAL_CCL_IMPORT);
    let resolvers: Vec<Arc<dyn ccl::ImportResolver>> = vec![
        Arc::new(static_resolver),
        Arc::new(ccl::FilesystemImportResolver::new()),
    ];

    let value = match ccl::exec_with_import_resolvers(ast, input, "", resolvers) {
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

    let mut out = metal_bus::Configuration::new();
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
    out: &mut metal_bus::Configuration,
) -> Result<(), MetalConfigError> {
    if let Some(ccl::Value::String(ty)) = dict.get("_metal_type") {
        return match ty.as_str() {
            "task" => extract_task(prefix, dict, out),
            "taskset" => extract_taskset(prefix, dict, out),
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

fn extract_taskset(
    prefix: &str,
    dict: &ccl::Dictionary,
    out: &mut metal_bus::Configuration,
) -> Result<(), MetalConfigError> {
    let mut inner_config = metal_bus::Configuration::new();
    let mut taskset = metal_bus::TaskSet::new();
    taskset.name = prefix.to_string();

    // Extract the inner content
    for (k, v) in &dict.kv_pairs {
        if let ccl::Value::Dictionary(dict) = v {
            if let Some(ccl::Value::String(ty)) = dict.get("_metal_type") {
                if ty.as_str() == "binding" {
                    taskset.service_bindings.push(extract_service_binding(
                        &join_prefix(prefix, k.as_str()),
                        dict,
                    )?);
                    continue;
                };
            }

            // If not a binding, perhaps it's another task
            extract_config(&join_prefix(prefix, k.as_str()), dict, &mut inner_config)?;
        }
    }

    for task in &inner_config.tasks {
        taskset.tasks.push(task.name.to_string());
    }

    out.tasksets.push(taskset);
    out.tasks.extend(inner_config.tasks);

    Ok(())
}

fn extract_task(
    prefix: &str,
    dict: &ccl::Dictionary,
    out: &mut metal_bus::Configuration,
) -> Result<(), MetalConfigError> {
    let mut task = metal_bus::Task::new();
    task.name = prefix.to_string();
    for (k, v) in &dict.kv_pairs {
        match k.as_str() {
            "binary" => match v {
                ccl::Value::Dictionary(bin) => task.binary = extract_binary(bin)?,
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "task's binary field must be a dictionary, got {:?}",
                        v.type_name()
                    )))
                }
            },
            "environment" => match v {
                ccl::Value::Dictionary(env) => task.environment = extract_environment(env)?,
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "task's environment field must be a dictionary, got {}",
                        v.type_name()
                    )))
                }
            },
            "arguments" => match v {
                ccl::Value::Array(arr) => {
                    task.arguments = extract_arguments(&arr)?;
                }
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "task's arguments field must be an array, got {}",
                        v.type_name()
                    )))
                }
            },
            "restart_mode" => match v {
                ccl::Value::String(v) => {
                    task.restart_mode = match v.as_str() {
                        "one_shot" => RestartMode::OneShot,
                        "on_failure" => RestartMode::OnFailure,
                        "always" => RestartMode::Always,
                        x => {
                            return Err(MetalConfigError::ConversionError(format!(
                                "restart_mode must be an one_shot, on_failure or always, got {}",
                                x
                            )))
                        }
                    }
                }
                x => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "restart_mode must be a string, got {}",
                        x.type_name()
                    )))
                }
            },
            _ => continue,
        }
    }

    // Validate task
    if task.binary.url.is_empty() && task.binary.path.is_empty() {
        return Err(MetalConfigError::ConversionError(String::from(
            "task must contain a binary!",
        )));
    }

    out.tasks.push(task);
    Ok(())
}

fn extract_binary(dict: &ccl::Dictionary) -> Result<metal_bus::Binary, MetalConfigError> {
    let mut binary = metal_bus::Binary::new();
    for (k, v) in &dict.kv_pairs {
        match k.as_str() {
            "url" => match v {
                ccl::Value::String(url) => binary.url = url.clone(),
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "binary's url field must be a string, got {:?}",
                        v.type_name()
                    )))
                }
            },
            "path" => match v {
                ccl::Value::String(path) => binary.path = path.clone(),
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "binary's path field must be a string, got {:?}",
                        v.type_name()
                    )))
                }
            },
            _ => continue,
        }
    }

    // Validate the binary
    if binary.url.is_empty() && binary.path.is_empty() {
        return Err(MetalConfigError::ConversionError(String::from(
            "binary must contain a url or path field!",
        )));
    }

    Ok(binary)
}

fn extract_service_binding(
    name: &str,
    dict: &ccl::Dictionary,
) -> Result<metal_bus::ServiceBinding, MetalConfigError> {
    let mut binding = metal_bus::ServiceBinding::new();
    binding.name = name.to_string();

    for (k, v) in &dict.kv_pairs {
        match k.as_str() {
            "port" => match v {
                ccl::Value::Number(num) => {
                    if *num <= 0.0 || *num > (u16::MAX as f64) {
                        return Err(MetalConfigError::ConversionError(format!(
                            "binding's port field must be a number between 1-65535, got {num}",
                        )));
                    }

                    binding.port = *num as u16;
                }
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "binding's port field must be a number, got {:?}",
                        v.type_name()
                    )))
                }
            },
            "hostname" => match v {
                ccl::Value::String(name) => {
                    binding.hostname = name.to_string();
                }
                _ => {
                    return Err(MetalConfigError::ConversionError(format!(
                        "binding's hostname field must be a string, got {:?}",
                        v.type_name()
                    )))
                }
            },
            _ => continue,
        }
    }

    // Validate the binary
    if binding.port == 0 || binding.hostname.is_empty() {
        return Err(MetalConfigError::ConversionError(String::from(
            "binary must contain a url or path field!",
        )));
    }

    Ok(binding)
}

fn extract_environment(
    dict: &ccl::Dictionary,
) -> Result<Vec<metal_bus::Environment>, MetalConfigError> {
    let mut environment = Vec::new();
    for (k, v) in &dict.kv_pairs {
        let mut env = metal_bus::Environment::new();
        env.name = k.clone();
        match v {
            ccl::Value::String(s) => {
                let mut av = ArgValue::new();
                av.value = s.clone();
                av.kind = ArgKind::String;
                env.value = av;
            }
            ccl::Value::Dictionary(dict) => {
                if let Some(ccl::Value::String(ty)) = dict.get("_metal_type") {
                    if ty.as_str() == "port" {
                        let mut av = ArgValue::new();
                        av.kind = ArgKind::PortAssignment;

                        if let Some(ccl::Value::String(service_name)) = dict.get("name") {
                            av.value = service_name.to_string();
                        } else {
                            return Err(MetalConfigError::ConversionError(format!(
                                "port binding must have a name",
                            )));
                        }

                        env.value = av;
                    } else if ty.as_str() == "secret" {
                        let mut av = ArgValue::new();
                        av.kind = ArgKind::Secret;

                        if let Some(ccl::Value::String(filename)) = dict.get("filename") {
                            av.value = filename.to_string();
                        } else {
                            return Err(MetalConfigError::ConversionError(format!(
                                "secret must have a filename",
                            )));
                        }
                        env.value = av;
                    } else {
                        return Err(MetalConfigError::ConversionError(format!(
                            "expected _metal_type port, got {}",
                            ty.as_str()
                        )));
                    }
                } else {
                    return Err(MetalConfigError::ConversionError(format!("expected environment value to be a string or a port_assignment, got dictionary")));
                }
            }
            _ => {
                return Err(MetalConfigError::ConversionError(format!(
                    "expected environment value to be a string or a port_assignment, got {}",
                    v.type_name()
                )))
            }
        };
        environment.push(env);
    }

    Ok(environment)
}

fn extract_arguments(args: &[ccl::Value]) -> Result<Vec<ArgValue>, MetalConfigError> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            ccl::Value::String(s) => {
                let mut arg = ArgValue::new();
                arg.value = s.clone();
                arg.kind = ArgKind::String;
                out.push(arg);
            }
            ccl::Value::Dictionary(dict) => {
                if let Some(ccl::Value::String(ty)) = dict.get("_metal_type") {
                    if ty == "port" {
                        let mut arg = ArgValue::new();
                        arg.kind = ArgKind::PortAssignment;
                        out.push(arg);
                    } else {
                        return Err(MetalConfigError::ConversionError(format!(
                            "unrecognized _metal_type {:?}",
                            ty
                        )));
                    }
                } else {
                    return Err(MetalConfigError::ConversionError(format!(
                        "task arguments must be a string or port assignment got dictionary",
                    )));
                }
            }
            _ => {
                return Err(MetalConfigError::ConversionError(format!(
                    "task arguments must be string or port assignment, got {}",
                    arg.type_name()
                )))
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_config() {
        let result = read_config(
            r#"
import { task, port_binding } from "metal"

namespace = {
    server = task {
        binary = {
            url = "http://test.com/server.exe"
        }
        environment = {
            SECRET_VALUE = "secret_content"
            PORT = port_binding {
                name = "my_service"
            }
        }
        arguments = [
            "--help", "xyz",
        ]
    }
}
"#,
        );
        if let Err(e) = &result {
            println!("got error: {:?}", e);
        }
        let r = result.unwrap();
        assert_eq!(r.tasks.len(), 1);
        let t = &r.tasks[0];
        assert_eq!(t.name, "namespace.server");
        assert_eq!(t.binary.url, "http://test.com/server.exe");
        assert_eq!(t.environment.len(), 2);
        assert_eq!(t.restart_mode, RestartMode::OnFailure);

        // NOTE: ccl dictionary values are sorted alphabetically
        assert_eq!(t.environment[0].name, "PORT");
        assert_eq!(t.environment[0].value.kind, ArgKind::PortAssignment);
        assert_eq!(t.environment[0].value.value, "my_service");

        assert_eq!(t.environment[1].name, "SECRET_VALUE");
        assert_eq!(t.environment[1].value.value, "secret_content");
        assert_eq!(t.environment[1].value.kind, ArgKind::String);

        assert_eq!(t.arguments.len(), 2);
        assert_eq!(t.arguments[0].value, "--help");
        assert_eq!(t.arguments[1].value, "xyz",);
    }

    #[test]
    fn test_read_taskset() {
        let result = read_config(
            r#"

import { secret, task, taskset, port_binding, service_binding } from "metal"

service = taskset {
    server = task {
        binary = {
            url = "http://test.com/server.exe"
        }
        environment = {
            VALUE = "1234"
            SECRET_VALUE = secret {
                filename = "/tmp/secret.txt"
            }
            PORT = port_binding {
                name = "http"
            }
        }
        arguments = [
            "--help", "xyz",
        ]
    }

    http = service_binding {
        hostname = "colinmerkel.xyz"
        port = 80
    }
}
"#,
        );
        if let Err(e) = &result {
            println!("got error: {:?}", e);
        }
        let r = result.unwrap();
        assert_eq!(r.tasks.len(), 1);
        let t = &r.tasks[0];
        assert_eq!(t.name, "service.server");
        assert_eq!(t.binary.url, "http://test.com/server.exe");
        assert_eq!(t.environment.len(), 3);
        assert_eq!(t.restart_mode, RestartMode::OnFailure);

        assert_eq!(r.tasksets.len(), 1);
        let t = &r.tasksets[0];
        assert_eq!(t.tasks, &["service.server"]);
        assert_eq!(t.service_bindings.len(), 1);
        assert_eq!(t.service_bindings[0].hostname, "colinmerkel.xyz");
        assert_eq!(t.service_bindings[0].port, 80);
        assert_eq!(t.service_bindings[0].name, "service.http");
    }
}

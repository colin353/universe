pub fn generate_config(text: &str) -> Result<x20::Configuration, String> {
    let parsed = match json::parse(text) {
        Ok(j) => j,
        Err(_) => return Err(String::from("unable to parse JSON")),
    };

    let mut config = x20::Configuration::new();
    if parsed["name"].is_null() || parsed["name"].as_str().unwrap().is_empty() {
        return Err(String::from("you must specify a non-empty name"));
    }
    config.set_name(parsed["name"].to_string());

    if parsed["environment"].is_null() || parsed["environment"].as_str().unwrap().is_empty() {
        return Err(String::from("you must specify a non-empty environment"));
    }
    config.set_environment(parsed["environment"].to_string());

    if parsed["priority"].is_null() || parsed["priority"].as_u64().is_none() {
        return Err(String::from("you must specify a numerical priority"));
    }
    config.set_priority(parsed["priority"].as_u64().unwrap());

    if parsed["binary_name"].is_null() || parsed["binary_name"].as_str().unwrap().is_empty() {
        return Err(String::from("you must specify a non-empty binary_name"));
    }
    config.set_binary_name(parsed["binary_name"].to_string());

    // Parse arguments
    if !parsed["arguments"].is_null() {
        for (k, v) in parsed["arguments"].entries() {
            let mut arg = x20::Argument::new();
            arg.set_name(k.to_string());
            arg.set_value(v.to_string());
            config.mut_arguments().push(arg);
        }
    }

    if !parsed["long_running"].is_null() && parsed["long_running"] == true {
        config.set_long_running(true);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_extraction() {
        let config = r#"
            { 
                "name": "boogaloo",
                "binary_name": "asdf",
                "environment": "australia",
                "priority": 250,
                "arguments": {
                    "port": 523
                }
            }
        "#;

        let mut expected = x20::Configuration::new();
        expected.set_name(String::from("boogaloo"));
        expected.set_binary_name(String::from("asdf"));
        expected.set_environment(String::from("australia"));
        expected.set_priority(250);

        let mut arg = x20::Argument::new();
        arg.set_name(String::from("port"));
        arg.set_value(String::from("523"));
        expected.mut_arguments().push(arg);

        assert_eq!(generate_config(config), Ok(expected));
    }
}

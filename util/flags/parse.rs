/*
 * Author:  colin.merkel@gmail.com
 * Date:    July 9 2017
 *
 * This file contains functions to parse command line strings
 * and recognize them for different types of flags.
 *
 */

use crate::ParseableFlag;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

// split takes an opt string and returns the name of the flag to
// be set, and the value assigned (or true, if nothing.)
fn split(opt: &str) -> Result<Option<(&str, &str)>, Error> {
    if !opt.starts_with("-") {
        return Ok(None);
    }

    // We want to allow you to use either double dash or single dash.
    let index = if opt.starts_with("--") { 2 } else { 1 };

    let flag = opt[index..].splitn(2, "=").collect::<Vec<_>>();

    // The flag name is the first value of the flag.
    let flag_name = flag[0];
    let flag_value = if flag.len() == 2 { flag[1] } else { "true" };

    // The flag name must contain only valid characters, which means lowercase
    // letters and underscores.
    for ch in flag_name.chars() {
        if ch != '_' && !ch.is_alphanumeric() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Flag `{}` contains invalid characters.", flag_name),
            ));
        }
    }

    Ok(Some((flag_name, flag_value)))
}

fn print_help_message(flags: &[&dyn ParseableFlag]) {
    println!("Options: ");
    for f in flags {
        println!(
            "\t--{:20} {} (default = {})\n",
            f.get_name(),
            f.get_usage_string(),
            f.get_default_value(),
        );
    }
}

// This function defines the format for environment arguments. For example,
// an environment argument like ARGS_FLAG_NAME will override a flag named
// --flag_name.
fn argname_from_envargname(envarg_name: &str) -> Option<String> {
    if !envarg_name.starts_with("ARGS_") {
        return None;
    }
    Some(envarg_name[5..].to_lowercase())
}

// This function does the opposite of argname_from_envargname(). It
// returns what we expect the environment variable name to be for a
// given argument name.
fn envargname_from_argname(arg_name: &str) -> String {
    format!("ARGS_{}", arg_name.to_uppercase())
}

// This function checks that the flags are parsed correctly, and errors
// if they are invalid, etc.
pub fn parse_flags_from_string(
    flags: &[&dyn ParseableFlag],
    args: &[&str],
    envargs: &[(&str, &str)],
) -> Result<Vec<String>, Error> {
    let mut m = HashMap::new();
    for f in flags {
        m.insert(f.get_name(), f);
    }

    let mut non_flag_arguments = Vec::<String>::new();

    for arg in args {
        let (name, value) = match split(arg) {
            Ok(Some((name, value))) => (name, value),
            // In this case, we're looking at a non-flag argument.
            Ok(None) => {
                non_flag_arguments.push((*arg).to_owned());
                continue;
            }
            Err(e) => return Err(e),
        };

        // If the user types "help", they probably want a list of all the
        // available flags, along with messages about each one.
        if name == "help" {
            print_help_message(flags);
            return Err(Error::new(
                ErrorKind::Interrupted,
                format!("User requested help message."),
            ));
        }

        match m.get(name) {
            Some(f) => match f.validate(value) {
                Ok(_) => continue,
                Err(e) => return Err(e),
            },
            None => {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Flag `{}` does not exist.", name),
                ))
            }
        };
    }

    // Environment variables override command line arguments. So we need to
    // make sure that they also parse correctly.
    for &(envarg_name, value) in envargs {
        let argname = match argname_from_envargname(envarg_name) {
            Some(x) => x,
            None => continue,
        };
        match m.get(argname.as_str()) {
            Some(f) => match f.validate(value) {
                Ok(_) => continue,
                Err(e) => return Err(e),
            },
            // If no match is found, it's probably just a spurious variable
            // which can be ignored without errors.
            None => continue,
        }
    }

    Ok(non_flag_arguments)
}

// get_flag_value looks through the provided arguments to find a flag with the same
// name as the provided flag, and if that exists, returns the associated value.
pub fn get_flag_value(flag_name: &str, args: &[&str], envargs: &[(&str, &str)]) -> Option<String> {
    let mut result = None;
    for arg in args {
        let (name, value) = match split(arg) {
            Ok(Some((name, value))) => (name, value),
            Ok(None) => continue,
            Err(_) => return None,
        };

        // Try to match the name of the flag with the name of the argument.
        if name == flag_name {
            result = Some(String::from(value));
            break;
        }
    }

    let envarg_formatted_name = envargname_from_argname(flag_name);
    for &(envarg_name, value) in envargs {
        if envarg_formatted_name == envarg_name {
            result = Some(value.to_owned());
        }
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Flag;

    #[test]
    fn test_parse_flags() {
        let boolFlag = Flag {
            name: "value",
            default: true,
            usage: "",
        };
        let intFlag = Flag {
            name: "number",
            default: 5,
            usage: "",
        };
        let flags: &[&dyn ParseableFlag] = &[&boolFlag, &intFlag];
        parse_flags_from_string(&flags, &["--value=true", "-number=3"], &[]).unwrap();

        assert_eq!(
            format!(
                "{}",
                parse_flags_from_string(flags, &["--value=yooo", "-number=3"], &[]).unwrap_err()
            ),
            "Flag `value`: couldn't parse value 'yooo'"
        );

        assert_eq!(
            format!(
                "{}",
                parse_flags_from_string(&flags, &["--nonflag=test", "-number=3"], &[]).unwrap_err()
            ),
            "Flag `nonflag` does not exist."
        );
    }

    #[test]
    fn test_nonflag_arguments() {
        let boolFlag = Flag {
            name: "value",
            default: true,
            usage: "",
        };
        let intFlag = Flag {
            name: "number",
            default: 5,
            usage: "",
        };
        let flags: &[&dyn ParseableFlag] = &[&boolFlag, &intFlag];
        assert_eq!(
            parse_flags_from_string(flags, &["file.txt", "-number=3"], &[]).unwrap(),
            vec!["file.txt"]
        );
    }

    #[test]
    fn test_get_boolean_flag() {
        // If a flag is present but has no value, it will be implicitly a boolean flag with value
        // set to "true"
        assert_eq!(
            get_flag_value("my_flag", &["--my_flag"], &[]).unwrap(),
            "true"
        );
    }

    #[test]
    fn test_get_flag_value() {
        // Environment flags should override command line flags.
        assert_eq!(
            get_flag_value("my_flag", &["--my_flag=true"], &[("ARGS_MY_FLAG", "env")]).unwrap(),
            "env"
        );

        // Should get a value if only environment flag is provided.
        assert_eq!(
            get_flag_value("my_flag", &[], &[("ARGS_MY_FLAG", "env")]).unwrap(),
            "env"
        );

        // Should just get the regular value even if extraneous args are provided.
        assert_eq!(
            get_flag_value(
                "my_flag",
                &["--my_flag=true"],
                &[("ARGS_MY_OTHEr_FLAG", "env")],
            )
            .unwrap(),
            "true"
        );
    }

    #[test]
    fn test_environment_args_parsing() {
        let boolFlag = Flag {
            name: "my_flag",
            default: true,
            usage: "My testing flag.",
        };
        parse_flags_from_string(
            &[&boolFlag],
            &["--my_flag=true"],
            &[("ARGS_MY_FLAG", "false")],
        )
        .unwrap();

        parse_flags_from_string(
            &[&boolFlag],
            &["--my_flag=true"],
            &[("ARGS_MY_FLAG", "fail")],
        )
        .unwrap_err();
    }

    #[test]
    fn test_envargname() {
        assert_eq!(argname_from_envargname("ARGS_NAME").unwrap(), "name");
        assert_eq!(argname_from_envargname("ANOTHER_ARG"), None);
    }

    #[test]
    fn test_argname() {
        assert_eq!(envargname_from_argname("name"), "ARGS_NAME");
        assert_eq!(envargname_from_argname("another_arg"), "ARGS_ANOTHER_ARG");
    }

    #[test]
    fn test_split() {
        assert_eq!(split("--hello=world").unwrap().unwrap(), ("hello", "world"));

        assert_eq!(split("-hello=world").unwrap().unwrap(), ("hello", "world"));

        assert_eq!(split("-hello").unwrap().unwrap(), ("hello", "true"));

        assert_eq!(
            split("--settings=--flag=false").unwrap().unwrap(),
            ("settings", "--flag=false",)
        );
    }

    #[test]
    fn test_invalid_flagname() {
        let e = split("--!@#=test").unwrap_err();
        assert_eq!(format!("{}", e), "Flag `!@#` contains invalid characters.")
    }
}

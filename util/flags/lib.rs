/*
 * Author:  colin.merkel@gmail.com
 * Date:    July 9 2017
 *
 * This module allows for simple flag definitions, similar to the way
 * it is done at Google. A macro is defined for creating a flag, e.g.
 *
 *  static flag: flags::Flag<bool> = define_flag!(flag_name, default_value);
 *
*/

use std::env;
use std::io::{Error, ErrorKind};

mod parse;

#[derive(Clone)]
pub struct Flag<T: std::str::FromStr> {
    pub name: &'static str,
    pub usage: &'static str,
    pub default: T,
}

pub trait ParseableFlag {
    fn validate(&self, &str) -> Result<(), Error>;
    fn get_name(&self) -> &str;
    fn get_usage_string(&self) -> &str;
    fn get_default_value(&self) -> String;
}

// parse_flags takes a set of flags and checks whether they are all present
// and whether they parse correctly.
pub fn parse_flags(flags: &[&ParseableFlag]) -> Result<Vec<String>, Error> {
    let args: Vec<String> = env::args().skip(1).collect();
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    parse::parse_flags_from_string(flags, &args_str, &[])
}

// parse_flags_or_panic tries to parse the flags, but if it fails, it checks the reason why and may
// panic and/or emit an error message. It returns a list of strings, which are the non-flag
// command-line arguments.
pub fn parse_flags_or_panic(flags: &[&ParseableFlag]) -> Vec<String> {
    let error = match parse_flags(flags) {
        Ok(args) => return args,
        Err(e) => e,
    };

    match error.kind() {
        ErrorKind::Interrupted => std::process::exit(1),
        _ => panic!(format!("{}", error)),
    }
}

impl<T: std::clone::Clone + std::str::FromStr + std::fmt::Display> ParseableFlag for Flag<T> {
    fn validate(&self, value: &str) -> Result<(), Error> {
        match self.parse(value) {
            Ok(_) => return Ok(()),
            Err(e) => return Err(e),
        }
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn get_usage_string(&self) -> &str {
        self.usage
    }

    fn get_default_value(&self) -> String {
        format!("{}", self.default)
    }
}

impl<T: std::clone::Clone + std::str::FromStr> Flag<T> {
    pub fn parse(&self, value: &str) -> Result<T, Error> {
        match value.parse() {
            Ok(x) => Ok(x),
            Err(_) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Flag `{}`: couldn't parse value '{}'", self.name, value),
            )),
        }
    }

    pub fn value(&self) -> T {
        let args: Vec<String> = env::args().skip(1).collect();
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let envargs: Vec<(String, String)> = env::vars().collect();
        let envargs_str: Vec<(&str, &str)> = envargs
            .iter()
            .map(|&(ref x, ref y)| (x.as_str(), y.as_str()))
            .collect();

        // Flags should already have been validated, so this should
        // never panic.
        match parse::get_flag_value(&self.name, &args_str, &envargs_str) {
            Some(value) => match value.parse() {
                Ok(v) => v,
                Err(_) => panic!("Flag `{}` couldn't be parsed.", self.name),
            },
            None => self.default.clone(),
        }
    }
}

#[macro_export]
macro_rules! define_flag {
    ($flag_name:expr, $default:expr, $usage:expr) => {
        flags::Flag {
            name: $flag_name,
            default: $default,
            usage: $usage,
        }
    };
}

#[macro_export]
macro_rules! parse_flags {
    ( $( $x: expr ),* ) => {
        flags::parse_flags_or_panic(vec![$(&$x as &flags::ParseableFlag,)*].as_slice())
    }
}

#[macro_export]
macro_rules! parse_module_flags {
    ( $( $x: expr ),*; $( $y: expr),* ) => {
        let mut flags_list: Vec<&flags::ParseableFlag> = vec![ $( &$x, )* ];
        $( flags_list.extend(&$y); )*
        flags::parse_flags_or_panic(&flags_list)
    }
}

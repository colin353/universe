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

pub trait FlagValue: Sized {
    fn from_str(s: &str) -> Result<Self, std::io::Error>;
}

impl FlagValue for String {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        Ok(s.to_string())
    }
}

impl FlagValue for u64 {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl FlagValue for i64 {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl FlagValue for u32 {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl FlagValue for i32 {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl FlagValue for u16 {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl FlagValue for u8 {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl FlagValue for usize {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl FlagValue for isize {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl FlagValue for bool {
    fn from_str(s: &str) -> Result<Self, std::io::Error> {
        s.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

// TODO: implement flags for Vec<...>?

#[derive(Clone)]
pub struct Flag<T: FlagValue> {
    pub name: &'static str,
    pub usage: &'static str,
    pub default: T,
}

pub trait ParseableFlag {
    fn validate(&self, _: &str) -> Result<(), Error>;
    fn get_name(&self) -> &str;
    fn get_usage_string(&self) -> &str;
    fn get_default_value(&self) -> String;
}

// parse_flags takes a set of flags and checks whether they are all present
// and whether they parse correctly.
pub fn parse_flags(flags: &[&dyn ParseableFlag]) -> Result<Vec<String>, Error> {
    let args: Vec<String> = env::args().skip(1).collect();
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    parse::parse_flags_from_string(flags, &args_str, &[])
}

// parse_flags_or_panic tries to parse the flags, but if it fails, it checks the reason why and may
// panic and/or emit an error message. It returns a list of strings, which are the non-flag
// command-line arguments.
pub fn parse_flags_or_panic(flags: &[&dyn ParseableFlag]) -> Vec<String> {
    let error = match parse_flags(flags) {
        Ok(args) => return args,
        Err(e) => e,
    };

    match error.kind() {
        ErrorKind::Interrupted => std::process::exit(1),
        _ => {
            eprintln!("{}", error);
            panic!("failed to parse flags!")
        }
    }
}

impl<T: std::clone::Clone + FlagValue + std::fmt::Display> ParseableFlag for Flag<T> {
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

impl<T: std::clone::Clone + FlagValue> Flag<T> {
    pub fn parse(&self, value: &str) -> Result<T, Error> {
        match T::from_str(value) {
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
            Some(value) => match T::from_str(value.as_str()) {
                Ok(v) => v,
                Err(_) => panic!("Flag `{}` couldn't be parsed.", self.name),
            },
            None => self.default.clone(),
        }
    }
}

impl Flag<String> {
    pub fn path(&self) -> String {
        let value = self.value();
        if value.starts_with("~/") {
            match env::var("HOME") {
                Ok(h) => return format!("{}{}", h, &value[1..]),
                Err(_) => (),
            };
        }
        value
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

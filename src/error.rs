#![macro_use]

use std::cmp::{min};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display};
use std::io::{Error};
use std::path::{PathBuf};

use colored::*;

use crate::config::*;
use crate::preprocess::*;

pub static mut WARNINGS_MAXIMUM: u32 = 10;
static mut WARNINGS_RAISED: Option<HashMap<String, u32>> = None;
pub static mut WARNINGS_MUTED: Option<HashSet<String>> = None;

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => (
        std::io::Error::new(std::io::ErrorKind::Other, format!($($arg)*))
    )
}

pub trait ErrorExt<T> {
    fn prepend_error<M: AsRef<[u8]> + Display>(self, msg: M) -> Result<T, Error>;
    fn print_error(self, exit: bool) -> ();
}
impl<T> ErrorExt<T> for Result<T, Error> {
    fn prepend_error<M: AsRef<[u8]> + Display>(self, msg: M) -> Result<T, Error> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(error!("{}\n{}", msg, e))
        }
    }

    fn print_error(self, exit: bool) {
        if let Err(error) = self {
            eprintln!("{}: {}", "error".red().bold(), error);

            if exit {
                print_warning_summary();
                std::process::exit(1);
            }
        }
    }
}

pub trait PreprocessParseErrorExt<T> {
    fn format_error(self, origin: &Option<PathBuf>, input: &str) -> Result<T, Error>;
}
impl<T> PreprocessParseErrorExt<T> for Result<T, preprocess_grammar::ParseError> {
    fn format_error(self, origin: &Option<PathBuf>, input: &str) -> Result<T, Error> {
        match self {
            Ok(t) => Ok(t),
            Err(pe) => {
                let line_origin = pe.line - 1;
                let file_origin = match origin {
                    Some(ref path) => format!("{}:", path.to_str().unwrap().to_string()),
                    None => "".to_string()
                };

                let line = input.lines().nth(pe.line - 1).unwrap_or("");

                Err(format_parse_error(line, file_origin, line_origin, pe.column, pe.expected))
            }
        }
    }
}

pub trait ConfigParseErrorExt<T> {
    fn format_error(self, info: &PreprocessInfo, input: &str) -> Result<T, Error>;
}
impl<T> ConfigParseErrorExt<T> for Result<T, config_grammar::ParseError> {
    fn format_error(self, info: &PreprocessInfo, input: &str) -> Result<T, Error> {
        match self {
            Ok(t) => Ok(t),
            Err(pe) => {
                let line_origin = info.line_origins[min(pe.line, info.line_origins.len()) - 1].0 as usize;
                let file_origin = match info.line_origins[min(pe.line, info.line_origins.len()) - 1].1 {
                    Some(ref path) => format!("{}:", path.to_str().unwrap().to_string()),
                    None => "".to_string()
                };

                let line = input.lines().nth(pe.line - 1).unwrap_or("");

                Err(format_parse_error(line, file_origin, line_origin, pe.column, pe.expected))
            }
        }
    }
}

fn format_parse_error(line: &str, file: String, line_number: usize, column_number: usize, expected: HashSet<&'static str>) -> Error {
    let trimmed = line.trim_start();
    let expected_list: Vec<String> = expected.iter().cloned().map(|x| format!("{:?}", x)).collect();

    error!("In line {}{}:\n\n  {}\n  {}{}\n\nUnexpected token \"{}\", expected: {}",
        file,
        line_number,
        trimmed,
        " ".to_string().repeat(column_number - 1 - (line.len() - trimmed.len())),
        "^".red().bold(),
        line.chars().map(|x| x.to_string()).nth(column_number - 1).unwrap_or_else(|| "\\n".to_string()),
        expected_list.join(", "))
}

pub fn warning<M: AsRef<[u8]> + Display>(msg: M, name: Option<&'static str>, location: (Option<M>,Option<u32>)) {
    unsafe {
        if WARNINGS_MUTED.is_none() {
            return;
        }

        if WARNINGS_RAISED.is_none() {
            WARNINGS_RAISED = Some(HashMap::new());
        }

        if let Some(name) = name {
            let raised = WARNINGS_RAISED.as_ref().unwrap().get(name).unwrap_or(&0);
            WARNINGS_RAISED.as_mut().unwrap().insert(name.to_string(), raised + 1);

            if raised >= &WARNINGS_MAXIMUM {
                return;
            }

            if WARNINGS_MUTED.as_ref().unwrap().contains(name) {
                return;
            }
        }
    }

    let loc_str = if location.0.is_some() && location.1.is_some() {
        format!("In file {}:{}: ", location.0.unwrap(), location.1.unwrap())
    } else if location.0.is_some() {
        format!("In file {}: ", location.0.unwrap())
    } else if location.1.is_some() {
        format!("In line {}: ", location.1.unwrap())
    } else {
        "".to_string()
    };

    let name_str = match name {
        Some(name) => format!(" [{}]", name),
        None => "".to_string()
    };

    eprintln!("{}{}: {}{}", loc_str, "warning".yellow().bold(), msg, name_str);
}

pub fn warning_suppressed(name: Option<&'static str>) -> bool {
    if name.is_none() {
        return false;
    }

    unsafe {
        if WARNINGS_MUTED.is_none() {
            return true;
        }

        if WARNINGS_MUTED.as_ref().unwrap().contains(name.unwrap()) {
            return true;
        }

        if WARNINGS_RAISED.is_some() {
            let raised = WARNINGS_RAISED.as_ref().unwrap().get(name.unwrap()).unwrap_or(&0);
            raised >= &WARNINGS_MAXIMUM
        } else {
            false
        }
    }
}

pub fn print_warning_summary() {
    unsafe {
        if WARNINGS_RAISED.is_none() || WARNINGS_MUTED.is_none() {
            return;
        }

        for (name, raised) in WARNINGS_RAISED.as_ref().unwrap().iter() {
            if WARNINGS_MUTED.as_ref().unwrap().contains(name) { continue; }

            if *raised <= WARNINGS_MAXIMUM { continue; }
            let excess = *raised - WARNINGS_MAXIMUM;

            if excess > 1 {
                warning(format!("{} warnings of type \"{}\" were suppressed to prevent spam. Use \"-w {}\" to disable these warnings entirely.",
                    excess, name, name), None, (None, None));
            } else {
                warning(format!("{} warning of type \"{}\" was suppressed to prevent spam. Use \"-w {}\" to disable these warnings entirely.",
                    excess, name, name), None, (None, None));
            }
        }
    }
}

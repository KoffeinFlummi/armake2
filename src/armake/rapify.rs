use std::io::{Read, Write};
use std::path::PathBuf;

use colored::*;

use armake::config::{Config};

pub fn cmd_rapify<I: Read, O: Write>(input: I, output: O, path: Option<PathBuf>) -> i32 {
    let config: Config;
    match Config::read(input, path) {
        Ok(cfg) => { config = cfg; },
        Err(msg) => {
            eprintln!("{} {}", "error:".red().bold(), msg);
            return 1;
        }
    }

    config.write_rapified(output).expect("Failed to write rapified config");

    0
}

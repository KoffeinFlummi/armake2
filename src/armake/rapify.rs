use std::io::{Read, Write};
use std::path::PathBuf;

use colored::*;
use time::precise_time_s;

use armake::config::{Config};

pub fn cmd_rapify<I: Read, O: Write>(input: &mut I, output: &mut O, path: Option<PathBuf>) -> i32 {
    let config: Config;
    match Config::read(input, path) {
        Ok(cfg) => { config = cfg; },
        Err(msg) => {
            eprintln!("{} {}", "error:".red().bold(), msg);
            return 1;
        }
    }

    let t = precise_time_s();

    config.write_rapified(output).expect("Failed to write rapified config");

    //println!("writing: {}", precise_time_s() - t);

    0
}

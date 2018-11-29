extern crate time;

use std::io::{Seek, Read, Write};

use armake::config::{Config};

pub fn cmd_derapify<I: Read + Seek, O: Write>(input: &mut I, output: &mut O) -> i32 {
    let mut config = Config::read_rapified(input).expect("Failed to read rapified config");

    config.write(output).expect("Failed to derapify config");

    0
}

use std::io::{Read, Write};

use armake::config::{Config};

pub fn cmd_rapify<I: Read + Clone, O: Write + Clone>(input: I, output: O) -> i32 {
    let mut config = Config::read(input).unwrap();

    config.derapify(output).expect("Failed to derapify config");

    0
}

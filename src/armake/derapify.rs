extern crate time;

use std::clone::Clone;
use std::io::{Seek, Read, Write};

use armake::config::{Config};

pub fn cmd_derapify<I: Read + Seek + Clone, O: Write + Clone>(input: I, output: O) -> i32 {
    let mut t = time::precise_time_s();

    let mut config = Config::read_rapified(input).expect("Failed to read rapified config");

    let t_read = time::precise_time_s() - t;
    t = time::precise_time_s();

    //println!("derapify");

    config.derapify(output).expect("Failed to derapify config");

    let t_write = time::precise_time_s() - t;

    //println!("read: {}ms, write: {}ms", t_read * 1000.0, t_write * 1000.0);

    0
}

pub mod error;
pub use crate::error::ArmakeError;

mod binarize;
pub use binarize::{binarize, find_binarize_exe};

mod config;
pub use config::Config;

pub mod commands;
pub use commands::Command;

pub mod pbo;
pub use pbo::{PBOHeader, PBO};

pub mod preprocess;

pub mod io;
use crate::io::{Input, Output};

#[cfg(feature = "signing")]
mod signing;
#[cfg(feature = "signing")]
pub use signing::{BIPrivateKey, BIPublicKey, BISign, BISignVersion};

use std::fs::File;
use std::io::{stdin, stdout, Cursor, Read};

fn get_input(source: Option<&str>) -> Result<Input, ArmakeError> {
    if let Some(ref path) = source {
        Ok(Input::File(File::open(path)?))
    } else {
        let mut buffer: Vec<u8> = Vec::new();
        stdin().read_to_end(&mut buffer).unwrap();
        Ok(Input::Cursor(Cursor::new(buffer.into_boxed_slice())))
    }
}

fn get_output(target: Option<&str>) -> Result<Output, ArmakeError> {
    if let Some(ref path) = target {
        Ok(Output::File(File::create(path)?))
    } else {
        Ok(Output::Standard(stdout()))
    }
}

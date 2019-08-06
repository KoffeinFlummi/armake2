use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::{ArmakeError, Command, binarize};
use crate::error;

pub struct Binarize {}
impl Binarize {
    /// Binarizes the given path using BI's binarize.exe (on Windows) and writes it to the output.
    fn cmd_binarize(input: PathBuf, output: PathBuf) -> Result<(), ArmakeError> {
        if !cfg!(windows) {
            return Err(error!("binarize.exe is only available on windows. Use rapify to binarize configs."));
        }

        let cursor = binarize(&input)?;
        let mut file = File::create(output)?;
        file.write_all(cursor.get_ref())?;

        Ok(())
    }
}

impl Command for Binarize {
    fn register(&self) -> clap::App {
        clap::SubCommand::with_name("binarize")
            .about("Binarize a file using BI's binarize.exe (Windows only)")
            .arg(clap::Arg::with_name("source")
                .help("Source file")
                .required(true)
            ).arg(clap::Arg::with_name("target")
                .help("Location to write file")
                .required(true)
            )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let input = args.value_of("source").unwrap();
        let output = args.value_of("target").unwrap();
        Binarize::cmd_binarize(PathBuf::from(input), PathBuf::from(output))
    }
}

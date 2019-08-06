use std::path::PathBuf;
use std::io::{Read, Write};

use crate::{ArmakeError, Command};
use crate::preprocess::preprocess;

pub struct Preprocess {}
impl Preprocess {
    pub fn cmd_preprocess<I: Read, O: Write>(input: &mut I, output: &mut O, path: Option<PathBuf>, includefolders: &[PathBuf]) -> Result<(), ArmakeError> {
        let mut buffer = String::new();
        input.read_to_string(&mut buffer)?;

        let (result, _) = preprocess(buffer, path, includefolders)?;

        output.write_all(result.as_bytes())?;

        Ok(())
    }
}

impl Command for Preprocess {
    fn register(&self) -> clap::App {
        clap::SubCommand::with_name("preprocess")
            .about("Preprocess a file")
            .arg(clap::Arg::with_name("source")
                .help("Source file")
                .required(true)
            ).arg(clap::Arg::with_name("target")
                .help("Location to write file")
            ).arg(clap::Arg::with_name("include")
                .help("Include folder")
                .short("i")
                .multiple(true)
                .takes_value(true)
            )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let mut input = crate::get_input(args.value_of("source"))?;
        let mut output = crate::get_output(args.value_of("target"))?;
        let includes: Vec<_> = args.values_of("include").unwrap().map(PathBuf::from).collect();
        Preprocess::cmd_preprocess(&mut input, &mut output, Some(PathBuf::from(args.value_of("source").unwrap())), &includes)
    }
}
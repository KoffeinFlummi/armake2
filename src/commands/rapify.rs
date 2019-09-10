use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;

use crate::{ArmakeError, Command, Config};

pub struct Rapify {}
impl Rapify {
    /// Reads input, preprocesses and rapifies it and writes to output.
    ///
    /// `path` is the path to the input if it is known and is used for relative includes and error
    /// messages. `includefolders` are the folders searched for absolute includes and should usually at
    /// least include the current working directory.
    fn cmd_rapify<I: Read, O: Write>(
        input: &mut I,
        output: &mut O,
        path: Option<PathBuf>,
        includefolders: &[PathBuf],
    ) -> Result<(), ArmakeError> {
        let config = Config::read(input, path, includefolders, |path| {
            let mut content = String::new();
            File::open(path)
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();
            content
        })?;

        config.write_rapified(output)?;

        Ok(())
    }
}

impl Command for Rapify {
    fn register(&self) -> clap::App {
        clap::SubCommand::with_name("rapify")
            .about("Preprocess and rapify a config file")
            .arg(
                clap::Arg::with_name("source")
                    .help("Source file")
                    .required(true),
            )
            .arg(clap::Arg::with_name("target").help("Location to write file"))
            .arg(
                clap::Arg::with_name("include")
                    .help("Include folder")
                    .short("i")
                    .multiple(true)
                    .takes_value(true),
            )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let mut input = crate::get_input(args.value_of("source"))?;
        let mut output = crate::get_output(args.value_of("target"))?;
        let includes: Vec<_> = args
            .values_of("include")
            .unwrap()
            .map(PathBuf::from)
            .collect();
        Rapify::cmd_rapify(
            &mut input,
            &mut output,
            Some(PathBuf::from(args.value_of("source").unwrap())),
            &includes,
        )
    }
}

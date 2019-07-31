use std::io::{Read, Seek, Write};

use crate::{ArmakeError, Command, Config};

pub struct Derapify {}
impl Derapify {
    /// Reads input, derapifies it and writes to output.
    pub fn cmd_derapify<I: Read + Seek, O: Write>(input: &mut I, output: &mut O) -> Result<(), ArmakeError> {
        let config = Config::read_rapified(input)?;

        config.write(output)?;

        Ok(())
    }
}

impl Command for Derapify {
    fn register(&self) -> (&str, clap::App) {
        ("derapify",
            clap::SubCommand::with_name("derapify")
                .about("Derapify a config")
                .arg(clap::Arg::with_name("source")
                    .help("Source file")
                    .required(true)
                ).arg(clap::Arg::with_name("target")
                    .help("Location to write file")
                    .required(true)
                )
        )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let mut input = crate::get_input(args.value_of("source"))?;
        let mut output = crate::get_output(args.value_of("target"))?;
        Derapify::cmd_derapify(&mut input, &mut output)
    }
}

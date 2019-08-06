use std::io::{Read, Write};

use crate::{ArmakeError, Command, PBO};

pub struct Cat {}
impl Cat {
    fn cmd_cat<I: Read, O: Write>(input: &mut I, output: &mut O, name: &str) -> Result<(), ArmakeError> {
        let pbo = PBO::read(input)?;

        match pbo.files.get(name) {
            Some(cursor) => {
                output.write_all(cursor.get_ref())?;
            },
            None => {
                eprintln!("not found"); // @todo
            }
        }

        Ok(())
    }
}

impl Command for Cat {
    fn register(&self) -> clap::App {
        clap::SubCommand::with_name("cat")
            .about("Read the named file from the target PBO")
            .arg(clap::Arg::with_name("source")
                .help("Target PBO to read")
                .required(true)
            ).arg(clap::Arg::with_name("filename")
                .help("File to read from PBO")
                .required(true)
            ).arg(clap::Arg::with_name("target")
                .help("Location to write file")
            )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let mut input = crate::get_input(args.value_of("source"))?;
        let mut output = crate::get_output(args.value_of("target"))?;
        let filename = args.value_of("filename").unwrap();
        Cat::cmd_cat(&mut input, &mut output, filename)
    }
}

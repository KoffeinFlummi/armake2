use std::io::Write;
use std::path::PathBuf;

use crate::{ArmakeError, Command, PBO};

pub struct Pack {}
impl Pack {
    pub fn cmd_pack<O: Write>(input: PathBuf, output: &mut O, headerext: &[&str], excludes: &[&str]) -> Result<(), ArmakeError> {
        let mut pbo = PBO::from_directory(input, false, excludes, &Vec::new())?;

        for h in headerext {
            let (key, value) = (h.split('=').nth(0).unwrap(), h.split('=').nth(1).unwrap());
            pbo.header_extensions.insert(key.to_string(), value.to_string());
        }

        pbo.write(output)?;

        Ok(())
    }
}

impl Command for Pack {
    fn register(&self) -> (&str, clap::App) {
        ("pack",
            clap::SubCommand::with_name("pack")
                .about("Pack a folder into a PBO without any binarization or rapification")
                .arg(clap::Arg::with_name("source")
                    .help("Source folder")
                    .required(true)
                ).arg(clap::Arg::with_name("target")
                    .help("Location to write file")
                ).arg(clap::Arg::with_name("header")
                    .help("Headers to add into the PBO")
                    .short("h")
                    .short("e")
                    .multiple(true)
                    .takes_value(true)
                ).arg(clap::Arg::with_name("exclude")
                    .help("Excluded files patterns")
                    .short("x")
                    .multiple(true)
                    .takes_value(true)
                )
        )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let input = args.value_of("source").unwrap();
        let mut output = crate::get_output(args.value_of("target"))?;
        let headers: Vec<_> = args.values_of("header").unwrap().collect();
        let excludes: Vec<_> = args.values_of("exclude").unwrap().collect();
        Pack::cmd_pack(PathBuf::from(input), &mut output, &headers, &excludes)
    }
}

use std::io::Write;
use std::path::PathBuf;

use crate::{ArmakeError, Command, PBO};

pub struct Build {}
impl Build {
    fn cmd_build<O: Write>(input: PathBuf, output: &mut O, headerext: &[&str], excludes: &[&str], includefolders: &[PathBuf]) -> Result<(), ArmakeError> {
        let mut pbo = PBO::from_directory(input, true, excludes, includefolders)?;

        for h in headerext {
            let (key, value) = (h.split('=').nth(0).unwrap(), h.split('=').nth(1).unwrap());
            pbo.header_extensions.insert(key.to_string(), value.to_string());
        }

        pbo.write(output)?;

        Ok(())
    }
}

impl Command for Build {
    fn register(&self) -> clap::App {
        clap::SubCommand::with_name("build")
            .about("Build a PBO from a folder")
            .arg(clap::Arg::with_name("source")
                .help("Source folder")
                .required(true)
            ).arg(clap::Arg::with_name("target")
                .help("Location to write file")
                .required(true)
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
            ).arg(clap::Arg::with_name("include")
                .help("Include folder")
                .short("i")
                .multiple(true)
                .takes_value(true)
            )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let input = args.value_of("source").unwrap();
        let mut output = crate::get_output(args.value_of("target"))?;
        let headers: Vec<_> = args.values_of("header").unwrap().collect();
        let excludes: Vec<_> = args.values_of("exclude").unwrap().collect();
        let includes: Vec<_> = args.values_of("include").unwrap().map(PathBuf::from).collect();
        Build::cmd_build(PathBuf::from(input), &mut output, &headers, &excludes, &includes)
    }
}

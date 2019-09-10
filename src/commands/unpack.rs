use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{PathBuf, MAIN_SEPARATOR};

use crate::{ArmakeError, Command, PBO};

pub struct Unpack {}
impl Unpack {
    fn cmd_unpack<I: Read>(input: &mut I, output: PathBuf) -> Result<(), ArmakeError> {
        let pbo = PBO::read(input)?;

        create_dir_all(&output)?;

        if !pbo.header_extensions.is_empty() {
            let prefix_path = output.join(PathBuf::from("$PBOPREFIX$"));
            let mut prefix_file = File::create(prefix_path)?;

            for (key, value) in pbo.header_extensions.iter() {
                prefix_file.write_all(format!("{}={}\n", key, value).as_bytes())?;
            }
        }

        for (file_name, cursor) in pbo.files.iter() {
            // @todo: windows
            let path = output.join(PathBuf::from(
                file_name.replace("\\", &MAIN_SEPARATOR.to_string()),
            ));
            create_dir_all(path.parent().unwrap())?;
            let mut file = File::create(path)?;
            file.write_all(cursor.get_ref())?;
        }

        Ok(())
    }
}

impl Command for Unpack {
    fn register(&self) -> clap::App {
        clap::SubCommand::with_name("unpack")
            .about("Unpack a PBO into a folder")
            .arg(
                clap::Arg::with_name("source")
                    .help("Source PBO file")
                    .required(true),
            )
            .arg(
                clap::Arg::with_name("target")
                    .help("Output folder")
                    .required(true),
            )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let mut input = crate::get_input(args.value_of("source"))?;
        let output = args.value_of("target").unwrap();
        Unpack::cmd_unpack(&mut input, PathBuf::from(output))
    }
}

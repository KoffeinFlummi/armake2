use std::io::Read;

use crate::{ArmakeError, Command, PBO};

pub struct Inspect {}
impl Inspect {
    fn cmd_inspect<I: Read>(input: &mut I) -> Result<(), ArmakeError> {
        let pbo = PBO::read(input)?;

        if !pbo.header_extensions.is_empty() {
            println!("Header extensions:");
            for (key, value) in pbo.header_extensions.iter() {
                println!("- {}={}", key, value);
            }
            println!();
        }

        println!("# Files: {}\n", pbo.files.len());

        println!("Path                                                  Method  Original    Packed");
        println!("                                                                  Size      Size");
        println!("================================================================================");
        for header in pbo.headers {
            println!("{:50} {:9} {:9} {:9}", header.filename, header.packing_method, header.original_size, header.data_size);
        }

        Ok(())
    }
}

impl Command for Inspect {
    fn register(&self) -> (&str, clap::App) {
        ("inspect",
            clap::SubCommand::with_name("inspect")
                .about("Inspect a PBO and list contained files")
                .arg(clap::Arg::with_name("source")
                    .help("Source file")
                    .required(true)
                )
        )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let mut input = crate::get_input(args.value_of("source"))?;
        Inspect::cmd_inspect(&mut input)    
    }
}

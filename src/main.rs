#[macro_use]
extern crate serde_derive;
extern crate docopt;
extern crate colored;
extern crate byteorder;
extern crate time;
extern crate openssl;

use std::io;
use std::io::{Read};
use std::path::PathBuf;
use std::env::current_dir;
use std::fs;

use docopt::Docopt;

mod armake;
use armake::io::{Input, Output};
use armake::preprocess;
use armake::rapify;
use armake::derapify;
use armake::pbo;

const USAGE: &'static str = "
armake2

Usage:
    armake2 build [-f] [-w <wname>]... [-i <includefolder>]... <sourcefolder> [<target>]
    armake2 pack [-f] <sourcefolder> [<target>]
    armake2 inspect [<source>]
    armake2 cat <source> <filename> [<target>]
    armake2 unpack [-f] <source> <targetfolder>
    armake2 preprocess [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 rapify [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 derapify [-f] [<source> [<target>]]
    armake2 (-h | --help)
    armake2 --version

Commands:
    inspect         Inspect a PBO.
    cat             Read a single file from a PBO.
    unpack          Unpack a PBO.
    preprocess      Preprocess a config.
    rapify          Preprocess & rapify a config.
    derapify        Derapify a config.

Options:
    -f --force                  Overwrite the target file/folder if it already exists.
    -w --warning <wname>        Warning to disable (repeatable).
    -i --include <includefolder>    Folder to search for includes, defaults to CWD (repeatable).
                                    For unpack: pattern to include in output folder (repeatable).
    -h --help                   Show usage information and exit.
    -v --version                Print the version number and exit.
";

#[derive(Debug, Deserialize)]
struct Args {
    cmd_build: bool,
    cmd_pack: bool,
    cmd_inspect: bool,
    cmd_cat: bool,
    cmd_unpack: bool,
    cmd_preprocess: bool,
    cmd_rapify: bool,
    cmd_derapify: bool,
    flag_version: bool,
    flag_force: bool,
    flag_warning: bool,
    flag_include: bool,
    arg_wname: Vec<String>,
    arg_includefolder: Vec<String>,
    arg_source: String,
    arg_target: String,
    arg_filename: String,
    arg_sourcefolder: String,
    arg_targetfolder: String,
}

fn get_input(args: &Args) -> Input {
    if args.arg_source == "" {
        let mut buffer: Vec<u8> = Vec::new();
        io::stdin().read_to_end(&mut buffer).unwrap();
        Input::Cursor(io::Cursor::new(buffer.into_boxed_slice()))
    } else {
        Input::File(fs::File::open(&args.arg_source).expect("Could not open input file"))
    }
}

fn get_output(args: &Args) -> Output {
    if args.arg_target == "" {
        Output::Standard(io::stdout())
    } else {
        Output::File(fs::File::create(&args.arg_target).expect("Could not open output file"))
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.deserialize())
                            .unwrap_or_else(|e| e.exit());

    //println!("{:?}", args);

    if args.flag_version {
        println!("v0.1.0");
        std::process::exit(0);
    }

    let path = if args.arg_source == "" {
        None
    } else {
        Some(PathBuf::from(&args.arg_source))
    };

    if args.cmd_rapify {
        std::process::exit(rapify::cmd_rapify(get_input(&args), get_output(&args), path));
    }

    if args.cmd_derapify {
        std::process::exit(derapify::cmd_derapify(&mut get_input(&args), &mut get_output(&args)));
    }

    if args.cmd_preprocess {
        std::process::exit(preprocess::cmd_preprocess(&mut get_input(&args), &mut get_output(&args), path));
    }

    if args.cmd_inspect {
        std::process::exit(pbo::cmd_inspect(&mut get_input(&args)));
    }

    if args.cmd_cat {
        std::process::exit(pbo::cmd_cat(&mut get_input(&args), &mut get_output(&args), args.arg_filename));
    }

    if args.cmd_unpack {
        std::process::exit(pbo::cmd_unpack(&mut get_input(&args), PathBuf::from(&args.arg_targetfolder)));
    }

    if args.cmd_pack {
        std::process::exit(pbo::cmd_pack(PathBuf::from(&args.arg_sourcefolder), &mut get_output(&args)));
    }

    unreachable!();
}

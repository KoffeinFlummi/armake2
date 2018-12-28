extern crate serde;
extern crate docopt;
extern crate colored;
extern crate byteorder;
extern crate time;
extern crate linked_hash_map;
extern crate openssl;
extern crate regex;

#[cfg(windows)]
extern crate winreg;

use std::io::{Error, Read, Cursor, stdin, stdout};
use std::path::{PathBuf};
use std::fs::{File};
use std::collections::{HashSet};
use std::iter::{FromIterator};

use serde::Deserialize;
use docopt::Docopt;

mod armake;
use armake::io::{Input, Output};
use armake::error::*;
use armake::config;
use armake::preprocess;
use armake::pbo;
use armake::sign;
use armake::binarize;

const USAGE: &'static str = "
armake2

Usage:
    armake2 rapify [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 preprocess [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 derapify [-v] [-f] [-d <indentation>] [<source> [<target>]]
    armake2 binarize [-v] [-f] [-w <wname>]... <source> <target>
    armake2 build [-v] [-f] [-w <wname>]... [-i <includefolder>]... [-x <excludepattern>]... [-e <headerext>]... [-k <privatekey>] [-s <signature>] <sourcefolder> [<target>]
    armake2 pack [-v] [-f] <sourcefolder> [<target>]
    armake2 inspect [-v] [<source>]
    armake2 unpack [-v] [-f] <source> <targetfolder>
    armake2 cat [-v] <source> <filename> [<target>]
    armake2 keygen [-v] [-f] <keyname>
    armake2 sign [-v] [-f] [-s <signature>] [--v2] <privatekey> <pbo> [<signature>]
    armake2 verify [-v] <publickey> <pbo> [<signature>]
    armake2 paa2img [-v] [-f] [<source> [<target>]]
    armake2 img2paa [-v] [-f] [-z] [-t <paatype>] [<source> [<target>]]
    armake2 (-h | --help)
    armake2 --version

Commands:
    rapify      Preprocess and rapify a config file.
    preprocess  Preprocess a file.
    derapify    Derapify a config.
    binarize    Binarize a file using BI's binarize.exe (Windows only).
    build       Build a PBO from a folder.
    pack        Pack a folder into a PBO without any binarization or rapification.
    inspect     Inspect a PBO and list contained files.
    unpack      Unpack a PBO into a folder.
    cat         Read the named file from the target PBO to stdout.
    keygen      Generate a keypair with the specified path (extensions are added).
    sign        Sign a PBO with the given private key.
    verify      Verify a PBO's signature with the given public key.
    paa2img     Convert PAA to image (PNG only). (not implemented)
    img2paa     Convert image to PAA. (not implemented)

Options:
    -v --verbose                Enable verbose output.
    -f --force                  Overwrite the target file/folder if it already exists.
    -w --warning <wname>        Warning to disable
    -i --include <includefolder>    Folder to search for includes, defaults to CWD.
    -x --exclude <excludepattern>   Glob pattern to exclude from PBO.
                                      For unpack: pattern to exclude from output folder.
    -d --indent <indentation>   String to use for indentation. 4 spaces by default.
    -e --headerext <headerext>  Extension to add to PBO header as \"key=value\".
    -k --key <privatekey>       Sign the PBO with the given private key.
    -s --signature <signature>  Signature path to use when signing the PBO.
       --v2                     Generate an older v2 signature.
    -z --compress               Compress final PAA where possible.
    -t --type <paatype>         PAA type. DXT1 or DXT5
    -h --help                   Show usage information and exit.
       --version                Print the version number and exit.
";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
struct Args {
    cmd_rapify: bool,
    cmd_preprocess: bool,
    cmd_derapify: bool,
    cmd_binarize: bool,
    cmd_build: bool,
    cmd_pack: bool,
    cmd_inspect: bool,
    cmd_unpack: bool,
    cmd_cat: bool,
    cmd_keygen: bool,
    cmd_sign: bool,
    cmd_verify: bool,
    cmd_paa2img: bool,
    cmd_img2paa: bool,
    flag_verbose: bool,
    flag_force: bool,
    flag_warning: Vec<String>,
    flag_include: Vec<String>,
    flag_exclude: Vec<String>,
    flag_headerext: Vec<String>,
    flag_key: Option<String>,
    flag_signature: Option<String>,
    flag_indent: Option<String>,
    flag_v2: bool,
    flag_compress: bool,
    flag_type: Option<String>,
    flag_version: bool,
    arg_wname: Vec<String>,
    arg_source: String,
    arg_target: String,
    arg_filename: String,
    arg_sourcefolder: String,
    arg_targetfolder: String,
    arg_keyname: String,
    arg_privatekey: String,
    arg_publickey: String,
    arg_signature: String,
    arg_pbo: String,
}

fn get_input(args: &Args) -> Result<Input, Error> {
    if args.arg_source == "" {
        let mut buffer: Vec<u8> = Vec::new();
        stdin().read_to_end(&mut buffer).unwrap();
        Ok(Input::Cursor(Cursor::new(buffer.into_boxed_slice())))
    } else {
        Ok(Input::File(File::open(&args.arg_source).prepend_error("Failed to open input file:")?))
    }
}

fn get_output(args: &Args) -> Result<Output, Error> {
    if args.arg_target == "" {
        Ok(Output::Standard(stdout()))
    } else {
        Ok(Output::File(File::create(&args.arg_target).prepend_error("Failed to open output file:")?))
    }
}

fn run_command(args: &Args) -> Result<(), Error> {
    let path = if args.arg_source == "" {
        None
    } else {
        Some(PathBuf::from(&args.arg_source))
    };

    let signature = if args.arg_signature == "" {
        None
    } else {
        Some(PathBuf::from(&args.arg_signature))
    };

    let mut includefolders: Vec<PathBuf> = args.flag_include.iter().map(|x| PathBuf::from(x)).collect();
    includefolders.push(PathBuf::from("."));

    if args.cmd_binarize {
        binarize::cmd_binarize(PathBuf::from(&args.arg_source), PathBuf::from(&args.arg_target))
    } else if args.cmd_rapify {
        config::cmd_rapify(&mut get_input(&args)?, &mut get_output(&args)?, path, &includefolders)
    } else if args.cmd_derapify {
        config::cmd_derapify(&mut get_input(&args)?, &mut get_output(&args)?)
    } else if args.cmd_preprocess {
        preprocess::cmd_preprocess(&mut get_input(&args)?, &mut get_output(&args)?, path, &includefolders)
    } else if args.cmd_build {
        pbo::cmd_build(PathBuf::from(&args.arg_sourcefolder), &mut get_output(&args)?, &args.flag_headerext, &args.flag_exclude, &includefolders)
    } else if args.cmd_pack {
        pbo::cmd_pack(PathBuf::from(&args.arg_sourcefolder), &mut get_output(&args)?, &args.flag_headerext, &args.flag_exclude)
    } else if args.cmd_inspect {
        pbo::cmd_inspect(&mut get_input(&args)?)
    } else if args.cmd_cat {
        pbo::cmd_cat(&mut get_input(&args)?, &mut get_output(&args)?, &args.arg_filename)
    } else if args.cmd_unpack {
        pbo::cmd_unpack(&mut get_input(&args)?, PathBuf::from(&args.arg_targetfolder))
    } else if args.cmd_keygen {
        sign::cmd_keygen(PathBuf::from(&args.arg_keyname))
    } else if args.cmd_sign {
        let version = if args.flag_v2 { sign::BISignVersion::V2 } else { sign::BISignVersion::V3 };
        sign::cmd_sign(PathBuf::from(&args.arg_privatekey), PathBuf::from(&args.arg_pbo), signature, version)
    } else if args.cmd_verify {
        sign::cmd_verify(PathBuf::from(&args.arg_publickey), PathBuf::from(&args.arg_pbo), signature)
    } else {
        unreachable!()
    }
}

fn main() {
    let mut args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.deserialize())
                            .unwrap_or_else(|e| e.exit());

    if args.flag_indent.is_none() {
        args.flag_indent = Some("    ".to_string());
    }

    //println!("{:?}", args);

    if args.flag_version {
        println!("v{}", VERSION);
        std::process::exit(0);
    }

    unsafe {
        WARNINGS_MUTED = Some(HashSet::from_iter(args.flag_warning.clone()));
    }

    run_command(&args).print_error(true);

    print_warning_summary();
}

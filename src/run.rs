use std::collections::{HashSet};
use std::fs::{File};
use std::io::{Error, Read, Cursor, stdin, stdout};
use std::iter::{FromIterator};
use std::path::{PathBuf};

use crate::*;
use crate::binarize;
use crate::config;
use crate::error::*;
use crate::io::{Input, Output};
use crate::paa;
use crate::pbo;
use crate::preprocess;
use crate::sign;

use serde::Deserialize;

pub const USAGE: &str = "
armake2

Usage:
    armake2 rapify [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 preprocess [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 derapify [-v] [-f] [-d <indentation>] [<source> [<target>]]
    armake2 binarize [-v] [-f] [-w <wname>]... <source> <target>
    armake2 build [-v] [-f] [--proton-binarize] [-w <wname>]... [-i <includefolder>]... [-x <excludepattern>]... [-e <headerext>]... [-k <privatekey>] [-s <signature>] <sourcefolder> [<target>]
    armake2 pack [-v] [-f] [-x <excludepattern>]... [-e <headerext>]... [-k <privatekey>] [-s <signature>] <sourcefolder> [<target>]
    armake2 inspect [-v] [<source>]
    armake2 unpack [-v] [-f] <source> <targetfolder>
    armake2 cat [-v] <source> <filename> [<target>]
    armake2 keygen [-v] [-f] <keyname>
    armake2 sign [-v] [-f] [--v2] <privatekey> <pbo> [<signature>]
    armake2 paa2img [-v] [-f] <source> <target>
    armake2 img2paa [-v] [-f] [-c] [-t <paatype>] <source> <target>
    armake2 verify [-v] <publickey> <pbo> [<signature>]
    armake2 (-h | --help)
    armake2 --version

Commands:
    rapify      Preprocess and rapify a config file.
    preprocess  Preprocess a file.
    derapify    Derapify a config.
    binarize    Binarize a file using BI's binarize.exe (Windows; on Linux through Proton).
                The model's folder is the addon and its name the prefix; use build
                for models whose textures are referenced by a longer prefix.
    build       Build a PBO from a folder.
    pack        Pack a folder into a PBO without any binarization or rapification.
    inspect     Inspect a PBO and list contained files.
    unpack      Unpack a PBO into a folder.
    cat         Read the named file from the target PBO to stdout.
    keygen      Generate a keypair with the specified path (extensions are added).
    sign        Sign a PBO with the given private key.
    verify      Verify a PBO's signature with the given public key.
    paa2img     Convert a PAA file to a standard image format (e.g. PNG).
    img2paa     Convert a standard image file to PAA.

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
    -c --compress               Compress the output PAA file.
    -t --type <paatype>         PAA compression type (e.g. dxt1, dxt5). [default: dxt5]
    -s --signature <signature>  Signature path to use when signing the PBO.
    --v2                     Generate an older v2 signature.
    --proton-binarize           Linux: binarize .p3d/.rtm with DayZ Tools' binarize.exe under Proton.
    -h --help                   Show usage information and exit.
       --version                Print the version number and exit.
";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize)]
pub struct Args {
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
    cmd_paa2img: bool,
    cmd_img2paa: bool,
    cmd_verify: bool,
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
    flag_proton_binarize: bool,
    flag_compress: bool,
    flag_type: String,
    flag_version: bool,
    arg_source: Option<String>,
    arg_target: Option<String>,
    arg_filename: String,
    arg_sourcefolder: String,
    arg_targetfolder: String,
    arg_keyname: String,
    arg_privatekey: String,
    arg_publickey: String,
    arg_signature: Option<String>,
    arg_pbo: String,
}

fn get_input(args: &Args) -> Result<Input, Error> {
    if let Some(ref source) = args.arg_source {
        Ok(Input::File(File::open(source).prepend_error("Failed to open input file:")?))
    } else {
        let mut buffer: Vec<u8> = Vec::new();
        stdin().read_to_end(&mut buffer).unwrap();
        Ok(Input::Cursor(Cursor::new(buffer.into_boxed_slice())))
    }
}

fn get_output(args: &Args) -> Result<Output, Error> {
    if let Some(ref target) = args.arg_target {
        let path = PathBuf::from(target);
        if !args.flag_force && path.exists() {
            return Err(error!("Target file \"{}\" already exists. Use --force to overwrite.", target));
        }
        Ok(Output::File(File::create(path).prepend_error("Failed to open output file:")?))
    } else {
        Ok(Output::Standard(stdout()))
    }
}

fn run_command(args: &Args) -> Result<(), Error> {
    let mut includefolders: Vec<PathBuf> = args.flag_include.iter().map(PathBuf::from).collect();
    includefolders.push(PathBuf::from("."));

    if args.cmd_binarize {
        binarize::cmd_binarize(PathBuf::from(args.arg_source.as_ref().unwrap()), PathBuf::from(args.arg_target.as_ref().unwrap()), args.flag_force)
    } else if args.cmd_rapify {
        config::cmd_rapify(&mut get_input(&args)?, &mut get_output(&args)?, args.arg_source.as_ref().map(PathBuf::from), &includefolders)
    } else if args.cmd_derapify {
        config::cmd_derapify(&mut get_input(&args)?, &mut get_output(&args)?)
    } else if args.cmd_preprocess {
        preprocess::cmd_preprocess(&mut get_input(&args)?, &mut get_output(&args)?, args.arg_source.as_ref().map(PathBuf::from), &includefolders)
    } else if args.cmd_build || args.cmd_pack {
        let flag_privatekey = args.flag_key.as_ref().map(PathBuf::from);
        let flag_signature = args.flag_signature.as_ref().map(PathBuf::from);

        if flag_privatekey.is_some() && args.arg_target.is_none() {
            return Err(error!("Cannot sign a pbo that is piped to stdout."));
        }

        if args.cmd_build {
            pbo::cmd_build(PathBuf::from(&args.arg_sourcefolder), &mut get_output(&args)?, &args.flag_headerext, &args.flag_exclude, &includefolders)?;
        } else {
            pbo::cmd_pack(PathBuf::from(&args.arg_sourcefolder), &mut get_output(&args)?, &args.flag_headerext, &args.flag_exclude)?;
        }

        if let Some(pkey) = flag_privatekey {
            sign::cmd_sign(pkey, PathBuf::from(args.arg_target.as_ref().unwrap()), flag_signature, sign::BISignVersion::V3, args.flag_force)?;
        }

        Ok(())
    } else if args.cmd_inspect {
        pbo::cmd_inspect(&mut get_input(&args)?)
    } else if args.cmd_cat {
        pbo::cmd_cat(&mut get_input(&args)?, &mut get_output(&args)?, &args.arg_filename)
    } else if args.cmd_unpack {
        pbo::cmd_unpack(&mut get_input(&args)?, PathBuf::from(&args.arg_targetfolder), args.flag_force)
    } else if args.cmd_keygen {
        sign::cmd_keygen(PathBuf::from(&args.arg_keyname), args.flag_force)
    } else if args.cmd_sign {
        let version = if args.flag_v2 { sign::BISignVersion::V2 } else { sign::BISignVersion::V3 };
        sign::cmd_sign(PathBuf::from(&args.arg_privatekey), PathBuf::from(&args.arg_pbo), args.arg_signature.as_ref().map(PathBuf::from), version, args.flag_force)
    } else if args.cmd_paa2img {
        paa::cmd_paa2img(PathBuf::from(args.arg_source.as_ref().unwrap()), PathBuf::from(args.arg_target.as_ref().unwrap()), args.flag_force)
    } else if args.cmd_img2paa {
        paa::cmd_img2paa(PathBuf::from(args.arg_source.as_ref().unwrap()), PathBuf::from(args.arg_target.as_ref().unwrap()), args.flag_type.clone(), args.flag_compress, args.flag_force)
    } else if args.cmd_verify {
        sign::cmd_verify(PathBuf::from(&args.arg_publickey), PathBuf::from(&args.arg_pbo), args.arg_signature.as_ref().map(PathBuf::from))
    } else {
        unreachable!()
    }
}

pub fn args(args: &mut Args) {
    if args.flag_indent.is_none() {
        args.flag_indent = Some("    ".to_string());
    }

    //println!("{:?}", args);

    if args.flag_version {
        println!("v{}", VERSION);
        std::process::exit(0);
    }

    binarize::set_proton(args.flag_proton_binarize);
    error::init_warnings(HashSet::from_iter(args.flag_warning.clone()), args.flag_verbose);
    let result = run_command(&args);
    binarize::cleanup(); // must happen before print_error, which exits on error
    result.print_error(true);

    print_warning_summary();
}

use std::fs::File;
use std::path::PathBuf;

use crate::{ArmakeError, BIPublicKey, BISign, Command, PBO};

pub struct Verify {}
impl Verify {
    pub fn cmd_verify(publickey_path: PathBuf, pbo_path: PathBuf, signature_path: Option<PathBuf>) -> Result<(), ArmakeError> {
        let publickey = BIPublicKey::read(&mut File::open(&publickey_path).expect("Failed to open public key")).expect("Failed to read public key");
        let pbo = PBO::read(&mut File::open(&pbo_path).expect("Failed to open PBO")).expect("Failed to read PBO");

        let sig_path = match signature_path {
            Some(path) => path,
            None => {
                let mut path = pbo_path.clone();
                path.set_extension(format!("pbo.{}.bisign", publickey.name));
                path
            }
        };

        let sig = BISign::read(&mut File::open(&sig_path).expect("Failed to open signature")).expect("Failed to read signature");

        publickey.verify(&pbo, &sig)
    }
}

impl Command for Verify {
    fn register(&self) -> (&str, clap::App) {
        ("verify",
            clap::SubCommand::with_name("verify")
                .about("Verify a PBO's signature with the given public key")
                .arg(clap::Arg::with_name("public")
                    .help("Public key (.bikey)")
                    .required(true)
                ).arg(clap::Arg::with_name("pbo")
                    .help("PBO file to verify")
                    .required(true)
                ).arg(clap::Arg::with_name("signature")
                    .help("Signature file (.bisign)")
                )
        )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let public = args.value_of("public").unwrap();
        let pbo = args.value_of("pbo").unwrap();
        let signature = args.value_of("signature").map_or_else(|| None, |o| Some(PathBuf::from(o)));
        Verify::cmd_verify(PathBuf::from(public), PathBuf::from(pbo), signature)
    }
}


use std::fs::File;
use std::path::PathBuf;

use crate::{ArmakeError, BIPrivateKey, BISignVersion, Command, PBO};

pub struct Sign {}
impl Sign {
    fn cmd_sign(
        privatekey_path: PathBuf,
        pbo_path: PathBuf,
        signature_path: Option<PathBuf>,
        version: BISignVersion,
    ) -> Result<(), ArmakeError> {
        let privatekey = BIPrivateKey::read(
            &mut File::open(&privatekey_path).expect("Failed to open private key"),
        )
        .expect("Failed to read private key");
        let pbo = PBO::read(&mut File::open(&pbo_path).expect("Failed to open PBO"))
            .expect("Failed to read PBO");

        let sig_path = match signature_path {
            Some(path) => path,
            None => {
                let mut path = pbo_path.clone();
                path.set_extension(format!("pbo.{}.bisign", privatekey.name));
                path
            }
        };

        let sig = privatekey.sign(&pbo, version);
        sig.write(&mut File::create(&sig_path).expect("Failed to open signature file"))
            .expect("Failed to write signature");

        Ok(())
    }
}

impl Command for Sign {
    fn register(&self) -> clap::App {
        clap::SubCommand::with_name("sign")
            .about("Sign a PBO with the given private key")
            .arg(
                clap::Arg::with_name("privatekey")
                    .help("Private key (.biprivatekey)")
                    .required(true),
            )
            .arg(
                clap::Arg::with_name("pbo")
                    .help("PBO to sign")
                    .required(true),
            )
            .arg(clap::Arg::with_name("signature").help("Filename of the output signature file"))
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let private = args.value_of("privatekey").unwrap();
        let pbo = args.value_of("pbo").unwrap();
        let signature = args
            .value_of("signature")
            .map_or_else(|| None, |o| Some(PathBuf::from(o)));
        Sign::cmd_sign(
            PathBuf::from(private),
            PathBuf::from(pbo),
            signature,
            BISignVersion::V3,
        )
    }
}

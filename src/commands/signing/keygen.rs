use std::fs::File;
use std::path::PathBuf;

use crate::{ArmakeError, BIPrivateKey, Command};

pub struct Keygen {}
impl Keygen {
    fn cmd_keygen(keyname: PathBuf) -> Result<(), ArmakeError> {
        let private_key = BIPrivateKey::generate(1024, keyname.file_name().unwrap().to_str().unwrap().to_string());
        let public_key = private_key.to_public_key();
        let name = keyname.file_name().unwrap().to_str().unwrap();

        let mut private_key_path = keyname.clone();
        private_key_path.set_file_name(format!("{}.biprivatekey", name));
        private_key.write(&mut File::create(private_key_path).unwrap()).expect("Failed to write private key");

        let mut public_key_path = keyname.clone();
        public_key_path.set_file_name(format!("{}.bikey", name));
        public_key.write(&mut File::create(public_key_path).unwrap()).expect("Failed to write public key");

        Ok(())
    }
}

impl Command for Keygen {
    fn register(&self) -> clap::App {
        clap::SubCommand::with_name("keygen")
            .about("Generate a keypair with the specified path (extensions are added)")
            .arg(clap::Arg::with_name("keyname")
                .help("Name of the keypair")
                .required(true)
            )
    }

    fn run(&self, args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        let output = args.value_of("keyname").unwrap();
        Keygen::cmd_keygen(PathBuf::from(output))
    }
}

use crate::ArmakeError;

pub trait Command {
    // (name, description)
    fn register(&self) -> clap::App;

    fn run(&self, _args: &clap::ArgMatches) -> Result<(), ArmakeError> {
        unimplemented!();
    }
}

mod inspect;
pub use inspect::Inspect;

mod cat;
pub use cat::Cat;

mod unpack;
pub use unpack::Unpack;

mod pack;
pub use pack::Pack;

mod build;
pub use build::Build;

mod binarize;
pub use binarize::Binarize;

mod rapify;
pub use rapify::Rapify;

mod derapify;
pub use derapify::Derapify;

mod preprocess;
pub use preprocess::Preprocess;

use armake2::*;
use docopt::Docopt;

use crate::run::{USAGE, Args};

fn main() {
    let mut args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.deserialize())
                            .unwrap_or_else(|e| e.exit());
    armake2::run::args(&mut args);
}

use docopt::Docopt;

use crate::run::{USAGE, Args};
use armake2::*;

fn main() {
    let mut args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.deserialize())
                            .unwrap_or_else(|e| e.exit());
    armake2::run::args(&mut args);
}

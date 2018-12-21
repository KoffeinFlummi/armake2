extern crate serde_derive;
extern crate docopt;
extern crate colored;
extern crate byteorder;
extern crate time;
extern crate linked_hash_map;
extern crate openssl;
extern crate regex;

#[cfg(windows)]
extern crate winreg;

mod armake;

pub use armake::{pbo,config,p3d,preprocess,binarize,sign,io};

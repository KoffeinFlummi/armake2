#[derive(Copy, Clone)]
pub enum BISignVersion {
    /// Version 2
    V2,
    /// Version 3
    V3,
}

impl Into<u32> for BISignVersion {
    fn into(self) -> u32 {
        match self {
            BISignVersion::V2 => 2,
            BISignVersion::V3 => 3,
        }
    }
}

mod private;
pub use private::BIPrivateKey;

mod public;
pub use public::BIPublicKey;

mod signature;
pub use signature::BISign;

use std::io::Cursor;

use openssl::bn::BigNum;
use openssl::hash::{DigestBytes, Hasher, MessageDigest};

use crate::PBO;

pub fn generate_hashes(pbo: &PBO, version: BISignVersion, length: u32) -> (BigNum, BigNum, BigNum) {
    let checksum = pbo.checksum.clone().unwrap();
    let hash1 = checksum.as_slice();

    let mut h = Hasher::new(MessageDigest::sha1()).unwrap();
    h.update(hash1).unwrap();
    h.update(&*namehash(pbo)).unwrap();
    if let Some(prefix) = pbo.header_extensions.get("prefix") {
        h.update(prefix.as_bytes()).unwrap();
        if !prefix.ends_with('\\') {
            h.update(b"\\").unwrap();
        }
    }
    let hash2 = &*h.finish().unwrap();

    h = Hasher::new(MessageDigest::sha1()).unwrap();
    h.update(&*filehash(pbo, version)).unwrap();
    h.update(&*namehash(pbo)).unwrap();
    if let Some(prefix) = pbo.header_extensions.get("prefix") {
        h.update(prefix.as_bytes()).unwrap();
        if !prefix.ends_with('\\') {
            h.update(b"\\").unwrap();
        }
    }
    let hash3 = &*h.finish().unwrap();

    (
        pad_hash(hash1, (length / 8) as usize),
        pad_hash(hash2, (length / 8) as usize),
        pad_hash(hash3, (length / 8) as usize),
    )
}

fn namehash(pbo: &PBO) -> DigestBytes {
    let mut files_sorted: Vec<(String, &Cursor<Box<[u8]>>)> = pbo
        .files
        .iter()
        .map(|(a, b)| (a.to_lowercase(), b))
        .collect();
    files_sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let mut h = Hasher::new(MessageDigest::sha1()).unwrap();

    for (name, data) in &files_sorted {
        if data.get_ref().len() == 0 {
            continue;
        }

        h.update(name.as_bytes()).unwrap();
    }

    h.finish().unwrap()
}

fn filehash(pbo: &PBO, version: BISignVersion) -> DigestBytes {
    let mut h = Hasher::new(MessageDigest::sha1()).unwrap();
    let mut nothing = true;

    for (name, cursor) in pbo.files.iter() {
        let ext = name.split('.').last().unwrap();

        match version {
            BISignVersion::V2 => {
                if ext == "paa"
                    || ext == "jpg"
                    || ext == "p3d"
                    || ext == "tga"
                    || ext == "rvmat"
                    || ext == "lip"
                    || ext == "ogg"
                    || ext == "wss"
                    || ext == "png"
                    || ext == "rtm"
                    || ext == "pac"
                    || ext == "fxy"
                    || ext == "wrp"
                {
                    continue;
                }
            }
            BISignVersion::V3 => {
                if ext != "sqf"
                    && ext != "inc"
                    && ext != "bikb"
                    && ext != "ext"
                    && ext != "fsm"
                    && ext != "sqm"
                    && ext != "hpp"
                    && ext != "cfg"
                    && ext != "sqs"
                    && ext != "h"
                {
                    continue;
                }
            }
        }

        h.update(cursor.get_ref()).unwrap();
        nothing = false;
    }

    match version {
        BISignVersion::V2 => {
            if nothing {
                h.update(b"nothing").unwrap();
            }
        }
        BISignVersion::V3 => {
            if nothing {
                h.update(b"gnihton").unwrap();
            }
        }
    }

    h.finish().unwrap()
}

fn pad_hash(hash: &[u8], size: usize) -> BigNum {
    let mut vec: Vec<u8> = Vec::new();

    vec.push(0);
    vec.push(1);
    vec.resize(size - 36, 255);
    vec.extend(b"\x00\x30\x21\x30\x09\x06\x05\x2b");
    vec.extend(b"\x0e\x03\x02\x1a\x05\x00\x04\x14");
    vec.extend(hash);

    BigNum::from_slice(&vec).unwrap()
}

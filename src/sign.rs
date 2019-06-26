//! Functions for creating and working with BI keys and signatures

use std::fs::{File};
use std::io::{Read, Write, Error, Cursor};
use std::path::{PathBuf};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use openssl::bn::{BigNum, BigNumContext};
use openssl::hash::{Hasher, MessageDigest, DigestBytes};
use openssl::rsa::{Rsa};

use crate::io::*;
use crate::pbo::*;

/// BI private key (.biprivatekey)
pub struct BIPrivateKey {
    name: String,
    length: u32,
    exponent: u32,
    n: BigNum,
    p: BigNum,
    q: BigNum,
    dmp1: BigNum,
    dmq1: BigNum,
    iqmp: BigNum,
    d: BigNum
}

/// BI public key (.bikey)
pub struct BIPublicKey {
    name: String,
    length: u32,
    exponent: u32,
    n: BigNum
}

/// BI signature version
#[derive(Copy,Clone)]
pub enum BISignVersion {
    /// Version 2
    V2,
    /// Version 3
    V3
}

/// BI signature (.bisign)
pub struct BISign {
    version: BISignVersion,
    name: String,
    length: u32,
    exponent: u32,
    n: BigNum,
    sig1: BigNum,
    sig2: BigNum,
    sig3: BigNum
}

fn write_bignum<O: Write>(output: &mut O, bn: &BigNum, size: usize) -> Result<(), Error> {
    let mut vec: Vec<u8> = bn.to_vec();
    vec = vec.iter().rev().cloned().collect();
    vec.resize(size, 0);

    Ok(output.write_all(&vec)?)
}

fn namehash(pbo: &PBO) -> DigestBytes {
    let mut files_sorted: Vec<(String,&Cursor<Box<[u8]>>)> = pbo.files.iter().map(|(a,b)| (a.to_lowercase(),b)).collect();
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
                if ext == "paa" || ext == "jpg" || ext == "p3d" ||
                    ext == "tga" || ext == "rvmat" || ext == "lip" ||
                    ext == "ogg" || ext == "wss" || ext == "png" ||
                    ext == "rtm" || ext == "pac" || ext == "fxy" ||
                    ext == "wrp" { continue; }
            },
            BISignVersion::V3 => {
                if ext != "sqf" && ext != "inc" && ext != "bikb" &&
                    ext != "ext" && ext != "fsm" && ext != "sqm" &&
                    ext != "hpp" && ext != "cfg" && ext != "sqs" &&
                    ext != "h" { continue; }
            }
        }

        h.update(cursor.get_ref()).unwrap();
        nothing = false;
    }

    match version {
        BISignVersion::V2 => if nothing { h.update(b"nothing").unwrap(); },
        BISignVersion::V3 => if nothing { h.update(b"gnihton").unwrap(); }
    }

    h.finish().unwrap()
}

fn generate_hashes(pbo: &PBO, version: BISignVersion, length: u32) -> (BigNum, BigNum, BigNum) {
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

    (pad_hash(hash1, (length / 8) as usize),
        pad_hash(hash2, (length / 8) as usize),
        pad_hash(hash3, (length / 8) as usize))
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

fn display_hashes(a: BigNum, b: BigNum) -> (String, String) {
    let hexa = a.to_hex_str().unwrap().to_lowercase();
    let hexb = b.to_hex_str().unwrap().to_lowercase();

    if hexa.len() != hexb.len() || hexa.len() <= 40 {
        return (hexa, hexb);
    }

    let (paddinga, hasha) = hexa.split_at(hexa.len() - 40);
    let (paddingb, hashb) = hexb.split_at(hexb.len() - 40);

    if paddinga != paddingb {
        (hexa, hexb)
    } else {
        (hasha.to_string(), hashb.to_string())
    }
}

impl BIPrivateKey {
    /// Reads a private key from the given input.
    pub fn read<I: Read>(input: &mut I) -> Result<BIPrivateKey, Error> {
        let name = input.read_cstring()?;
        let temp = input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        let length = input.read_u32::<LittleEndian>()?;
        let exponent = input.read_u32::<LittleEndian>()?;

        assert_eq!(temp, length / 16 * 9 + 20);

        let mut buffer = vec![0; (length / 8) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let n = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let p = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let q = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let dmp1 = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let dmq1 = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let iqmp = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 8) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let d = BigNum::from_slice(&buffer).unwrap();

        Ok(BIPrivateKey {
            name,
            length,
            exponent,
            n,
            p,
            q,
            dmp1,
            dmq1,
            iqmp,
            d,
        })
    }

    /// Generate a new private key with the given name and bitlength.
    ///
    /// Arma 3 uses 1024 bit keys.
    pub fn generate(length: u32, name: String) -> BIPrivateKey {
        let rsa = Rsa::generate(length).expect("Failed to generate keypair");

        BIPrivateKey {
            name,
            length,
            exponent: 65537,
            n: BigNum::from_slice(&rsa.n().to_vec()).unwrap(),
            p: BigNum::from_slice(&rsa.p().unwrap().to_vec()).unwrap(),
            q: BigNum::from_slice(&rsa.q().unwrap().to_vec()).unwrap(),
            dmp1: BigNum::from_slice(&rsa.dmp1().unwrap().to_vec()).unwrap(),
            dmq1: BigNum::from_slice(&rsa.dmq1().unwrap().to_vec()).unwrap(),
            iqmp: BigNum::from_slice(&rsa.iqmp().unwrap().to_vec()).unwrap(),
            d: BigNum::from_slice(&rsa.d().to_vec()).unwrap(),
        }
    }

    /// Returns the public key for this private key.
    pub fn to_public_key(&self) -> BIPublicKey {
        BIPublicKey {
            name: self.name.clone(),
            length: self.length,
            exponent: self.exponent,
            n: BigNum::from_slice(&self.n.to_vec()).unwrap(),
        }
    }

    /// Signs the given PBO with this private key.
    pub fn sign(&self, pbo: &PBO, version: BISignVersion) -> BISign {
        let (hash1, hash2, hash3) = generate_hashes(pbo, version, self.length);

        let mut ctx = BigNumContext::new().unwrap();

        let mut sig1: BigNum = BigNum::new().unwrap();
        sig1.mod_exp(&hash1, &self.d, &self.n, &mut ctx).unwrap();
        let mut sig2: BigNum = BigNum::new().unwrap();
        sig2.mod_exp(&hash2, &self.d, &self.n, &mut ctx).unwrap();
        let mut sig3: BigNum = BigNum::new().unwrap();
        sig3.mod_exp(&hash3, &self.d, &self.n, &mut ctx).unwrap();

        BISign {
            version,
            name: self.name.clone(),
            length: self.length,
            exponent: self.exponent,
            n: BigNum::from_slice(&self.n.to_vec()).unwrap(),
            sig1,
            sig2,
            sig3,
        }
    }

    /// Write private key to output.
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        output.write_cstring(&self.name)?;
        output.write_u32::<LittleEndian>(self.length / 16 * 9 + 20)?;
        output.write_all(b"\x07\x02\x00\x00\x00\x24\x00\x00")?;
        output.write_all(b"RSA2")?;
        output.write_u32::<LittleEndian>(self.length)?;
        output.write_u32::<LittleEndian>(self.exponent)?;
        write_bignum(output, &self.n, (self.length / 8) as usize)?;
        write_bignum(output, &self.p, (self.length / 16) as usize)?;
        write_bignum(output, &self.q, (self.length / 16) as usize)?;
        write_bignum(output, &self.dmp1, (self.length / 16) as usize)?;
        write_bignum(output, &self.dmq1, (self.length / 16) as usize)?;
        write_bignum(output, &self.iqmp, (self.length / 16) as usize)?;
        write_bignum(output, &self.d, (self.length / 8) as usize)?;
        Ok(())
    }
}

impl BIPublicKey {
    /// Reads a public key from the given input.
    pub fn read<I: Read>(input: &mut I) -> Result<BIPublicKey, Error> {
        let name = input.read_cstring()?;
        let temp = input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        let length = input.read_u32::<LittleEndian>()?;
        let exponent = input.read_u32::<LittleEndian>()?;

        assert_eq!(temp, length / 8 + 20);

        let mut buffer = vec![0; (length / 8) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let n = BigNum::from_slice(&buffer).unwrap();

        Ok(BIPublicKey {
            name,
            length,
            exponent,
            n,
        })
    }

    // @todo: example
    /// Verifies a signature against this public key.
    pub fn verify(&self, pbo: &PBO, signature: &BISign) -> Result<(), Error> {
        let (real_hash1, real_hash2, real_hash3) = generate_hashes(pbo, signature.version, self.length);

        let mut ctx = BigNumContext::new().unwrap();

        let exponent = BigNum::from_u32(self.exponent).unwrap();

        let mut signed_hash1: BigNum = BigNum::new().unwrap();
        signed_hash1.mod_exp(&signature.sig1, &exponent, &self.n, &mut ctx).unwrap();
        let mut signed_hash2: BigNum = BigNum::new().unwrap();
        signed_hash2.mod_exp(&signature.sig2, &exponent, &self.n, &mut ctx).unwrap();
        let mut signed_hash3: BigNum = BigNum::new().unwrap();
        signed_hash3.mod_exp(&signature.sig3, &exponent, &self.n, &mut ctx).unwrap();

        if real_hash1 != signed_hash1 {
            let (s, r) = display_hashes(signed_hash1, real_hash1);
            return Err(error!("Hash 1 doesn't match\nSigned hash: {}\nReal hash:   {}", s, r));
        }

        if real_hash2 != signed_hash2 {
            let (s, r) = display_hashes(signed_hash2, real_hash2);
            return Err(error!("Hash 2 doesn't match\nSigned hash: {}\nReal hash:   {}", s, r));
        }

        if real_hash3 != signed_hash3 {
            let (s, r) = display_hashes(signed_hash3, real_hash3);
            return Err(error!("Hash 3 doesn't match\nSigned hash: {}\nReal hash:   {}", s, r));
        }

        Ok(())
    }

    /// Write public key to output.
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        output.write_cstring(&self.name)?;
        output.write_u32::<LittleEndian>(self.length / 8 + 20)?;
        output.write_all(b"\x06\x02\x00\x00\x00\x24\x00\x00")?;
        output.write_all(b"RSA1")?;
        output.write_u32::<LittleEndian>(self.length)?;
        output.write_u32::<LittleEndian>(self.exponent)?;
        write_bignum(output, &self.n, (self.length / 8) as usize)?;
        Ok(())
    }
}

impl Into<u32> for BISignVersion {
    fn into(self) -> u32 {
        match self {
            BISignVersion::V2 => 2,
            BISignVersion::V3 => 3,
        }
    }
}

/// BI signature (.bisign)
impl BISign {
    /// Reads a signature from the given input.
    pub fn read<I: Read>(input: &mut I) -> Result<BISign, Error> {
        let name = input.read_cstring()?;
        let temp = input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        input.read_u32::<LittleEndian>()?;
        let length = input.read_u32::<LittleEndian>()?;
        let exponent = input.read_u32::<LittleEndian>()?;

        assert_eq!(temp, length / 8 + 20);

        let mut buffer = vec![0; (length / 8) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let n = BigNum::from_slice(&buffer).unwrap();

        input.read_u32::<LittleEndian>()?;

        let mut buffer = vec![0; (length / 8) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let sig1 = BigNum::from_slice(&buffer).unwrap();

        let version = match input.read_u32::<LittleEndian>()? {
            2 => BISignVersion::V2,
            3 => BISignVersion::V3,
            _ => {
                return Err(error!("Unknown BISign version."));
            }
        };

        input.read_u32::<LittleEndian>()?;

        let mut buffer = vec![0; (length / 8) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let sig2 = BigNum::from_slice(&buffer).unwrap();

        input.read_u32::<LittleEndian>()?;

        let mut buffer = vec![0; (length / 8) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().cloned().collect();
        let sig3 = BigNum::from_slice(&buffer).unwrap();

        Ok(BISign {
            version,
            name,
            length,
            exponent,
            n,
            sig1,
            sig2,
            sig3,
        })
    }

    /// Writes the signature to the given output.
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        output.write_cstring(&self.name)?;
        output.write_u32::<LittleEndian>(self.length / 8 + 20)?;
        output.write_all(b"\x06\x02\x00\x00\x00\x24\x00\x00")?;
        output.write_all(b"RSA1")?;
        output.write_u32::<LittleEndian>(self.length)?;
        output.write_u32::<LittleEndian>(self.exponent)?;
        write_bignum(output, &self.n, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        write_bignum(output, &self.sig1, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(self.version.into())?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        write_bignum(output, &self.sig2, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        write_bignum(output, &self.sig3, (self.length / 8) as usize)?;
        Ok(())
    }
}

/// Generates a key pair with the given name.
///
/// The output paths are created by appending extensions to the keyname.
pub fn cmd_keygen(keyname: PathBuf) -> Result<(), Error> {
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

/// Signs a PBO with the given private key.
///
/// If the signature path is not given it is inferred from the PBO path.
pub fn cmd_sign(privatekey_path: PathBuf, pbo_path: PathBuf, signature_path: Option<PathBuf>, version: BISignVersion) -> Result<(), Error> {
    let privatekey = BIPrivateKey::read(&mut File::open(&privatekey_path).expect("Failed to open private key")).expect("Failed to read private key");
    let pbo = PBO::read(&mut File::open(&pbo_path).expect("Failed to open PBO")).expect("Failed to read PBO");

    let sig_path = match signature_path {
        Some(path) => path,
        None => {
            let mut path = pbo_path.clone();
            path.set_extension(format!("pbo.{}.bisign", privatekey.name));
            path
        }
    };

    let sig = privatekey.sign(&pbo, version);
    sig.write(&mut File::create(&sig_path).expect("Failed to open signature file")).expect("Failed to write signature");

    Ok(())
}

/// Verifies a signature for a pbo against a given public key.
///
/// If the signature path is not given it is inferred from the PBO path.
pub fn cmd_verify(publickey_path: PathBuf, pbo_path: PathBuf, signature_path: Option<PathBuf>) -> Result<(), Error> {
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

use std::io::{Read, Write, Error};
use std::fs::{File};
use std::path::{PathBuf};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use openssl::hash::{Hasher, MessageDigest};
use openssl::bn::{BigNum, BigNumContext};
use openssl::rsa::{Rsa};

use armake::io::*;
use armake::pbo::*;

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

pub struct BIPublicKey {
    name: String,
    length: u32,
    exponent: u32,
    n: BigNum
}

pub struct BISign {
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
    vec.resize(size, 0);

    vec = vec.iter().rev().map(|x| *x).collect();

    Ok(output.write_all(&vec)?)
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

impl BIPrivateKey {
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
        buffer = buffer.iter().rev().map(|x| *x).collect();
        let n = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().map(|x| *x).collect();
        let p = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().map(|x| *x).collect();
        let q = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().map(|x| *x).collect();
        let dmp1 = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().map(|x| *x).collect();
        let dmq1 = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 16) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().map(|x| *x).collect();
        let iqmp = BigNum::from_slice(&buffer).unwrap();

        buffer = vec![0; (length / 8) as usize];
        input.read_exact(&mut buffer)?;
        buffer = buffer.iter().rev().map(|x| *x).collect();
        let d = BigNum::from_slice(&buffer).unwrap();

        Ok(BIPrivateKey {
            name: name,
            length: length,
            exponent: exponent,
            n: n,
            p: p,
            q: q,
            dmp1: dmp1,
            dmq1: dmq1,
            iqmp: iqmp,
            d: d
        })
    }

    pub fn generate(length: u32, name: String) -> BIPrivateKey {
        let rsa = Rsa::generate(length).expect("Failed to generate keypair");

        BIPrivateKey {
            name: name,
            length: length,
            exponent: 65537,
            n: BigNum::from_slice(&rsa.n().to_vec()).unwrap(),
            p: BigNum::from_slice(&rsa.p().unwrap().to_vec()).unwrap(),
            q: BigNum::from_slice(&rsa.q().unwrap().to_vec()).unwrap(),
            dmp1: BigNum::from_slice(&rsa.dmp1().unwrap().to_vec()).unwrap(),
            dmq1: BigNum::from_slice(&rsa.dmq1().unwrap().to_vec()).unwrap(),
            iqmp: BigNum::from_slice(&rsa.iqmp().unwrap().to_vec()).unwrap(),
            d: BigNum::from_slice(&rsa.d().to_vec()).unwrap()
        }
    }

    pub fn to_public_key(&self) -> BIPublicKey {
        BIPublicKey {
            name: self.name.clone(),
            length: self.length,
            exponent: self.exponent,
            n: BigNum::from_slice(&self.n.to_vec()).unwrap()
        }
    }

    pub fn sign(&self, pbo: &PBO) -> BISign {
        let checksum = pbo.checksum.clone().unwrap();
        let hash1 = checksum.as_slice();

        let mut h = Hasher::new(MessageDigest::sha1()).unwrap();
        h.update(hash1).unwrap();
        h.update(&*pbo.namehash()).unwrap();
        if let Some(prefix) = pbo.header_extensions.get("prefix") {
            h.update(prefix.as_bytes()).unwrap();
            if prefix.chars().last().unwrap() != '\\' {
                h.update(b"\\").unwrap();
            }
        }
        let hash2 = &*h.finish().unwrap();

        h = Hasher::new(MessageDigest::sha1()).unwrap();
        h.update(&*pbo.filehash()).unwrap();
        h.update(&*pbo.namehash()).unwrap();
        if let Some(prefix) = pbo.header_extensions.get("prefix") {
            h.update(prefix.as_bytes()).unwrap();
            if prefix.chars().last().unwrap() != '\\' {
                h.update(b"\\").unwrap();
            }
        }
        let hash3 = &*h.finish().unwrap();

        let hash1_padded = pad_hash(hash1, (self.length / 8) as usize);
        let hash2_padded = pad_hash(hash2, (self.length / 8) as usize);
        let hash3_padded = pad_hash(hash3, (self.length / 8) as usize);

        let mut ctx = BigNumContext::new().unwrap();

        let mut sig1: BigNum = BigNum::new().unwrap();
        sig1.mod_exp(&hash1_padded, &self.d, &self.n, &mut ctx).unwrap();
        let mut sig2: BigNum = BigNum::new().unwrap();
        sig2.mod_exp(&hash2_padded, &self.d, &self.n, &mut ctx).unwrap();
        let mut sig3: BigNum = BigNum::new().unwrap();
        sig3.mod_exp(&hash3_padded, &self.d, &self.n, &mut ctx).unwrap();

        BISign {
            name: self.name.clone(),
            length: self.length,
            exponent: self.exponent,
            n: BigNum::from_slice(&self.n.to_vec()).unwrap(),
            sig1: sig1,
            sig2: sig2,
            sig3: sig3
        }
    }

    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        output.write_all(self.name.as_bytes())?;
        output.write_all(b"\0")?;
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
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        output.write_all(self.name.as_bytes())?;
        output.write_all(b"\0")?;
        output.write_u32::<LittleEndian>(self.length / 8 + 20)?;
        output.write_all(b"\x06\x02\x00\x00\x00\x24\x00\x00")?;
        output.write_all(b"RSA1")?;
        output.write_u32::<LittleEndian>(self.length)?;
        output.write_u32::<LittleEndian>(self.exponent)?;
        write_bignum(output, &self.n, (self.length / 8) as usize)?;
        Ok(())
    }
}

impl BISign {
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        output.write_all(self.name.as_bytes())?;
        output.write_all(b"\0")?;
        output.write_u32::<LittleEndian>(self.length / 8 + 20)?;
        output.write_all(b"\x06\x02\x00\x00\x00\x24\x00\x00")?;
        output.write_all(b"RSA1")?;
        output.write_u32::<LittleEndian>(self.length)?;
        output.write_u32::<LittleEndian>(self.exponent)?;
        write_bignum(output, &self.n, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        write_bignum(output, &self.sig1, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(2)?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        write_bignum(output, &self.sig2, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        write_bignum(output, &self.sig3, (self.length / 8) as usize)?;
        Ok(())
    }
}

pub fn cmd_keygen(keyname: PathBuf) -> i32 {
    let private_key = BIPrivateKey::generate(1024, keyname.file_name().unwrap().to_str().unwrap().to_string());
    let public_key = private_key.to_public_key();

    let mut private_key_path = keyname.clone();
    private_key_path.set_extension("biprivatekey");
    private_key.write(&mut File::create(private_key_path).unwrap()).expect("Failed to write private key");

    let mut public_key_path = keyname.clone();
    public_key_path.set_extension("bipublickey");
    public_key.write(&mut File::create(public_key_path).unwrap()).expect("Failed to write public key");

    0
}

pub fn cmd_sign(privatekey_path: PathBuf, pbo_path: PathBuf) -> i32 {
    let privatekey = BIPrivateKey::read(&mut File::open(&privatekey_path).expect("Failed to open private key")).expect("Failed to read private key");
    let pbo = PBO::read(&mut File::open(&pbo_path).expect("Failed to open PBO")).expect("Failed to read PBO");

    let mut sig_path = pbo_path.clone();
    sig_path.set_extension(format!("pbo.{}.bisign", privatekey.name));

    let sig = privatekey.sign(&pbo);
    sig.write(&mut File::create(&sig_path).expect("Failed to open signature file")).expect("Failed to write signature");

    0
}

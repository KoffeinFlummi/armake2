use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use openssl::bn::{BigNum, BigNumContext};
use openssl::rsa::Rsa;

use crate::{ArmakeError, BIPublicKey, BISign, BISignVersion, PBO};
use crate::io::{ReadExt, WriteExt};

pub struct BIPrivateKey {
    pub name: String,
    pub length: u32,
    pub exponent: u32,
    pub n: BigNum,
    pub p: BigNum,
    pub q: BigNum,
    pub dmp1: BigNum,
    pub dmq1: BigNum,
    pub iqmp: BigNum,
    pub d: BigNum
}

impl BIPrivateKey {
    /// Reads a private key from the given input.
    pub fn read<I: Read>(input: &mut I) -> Result<BIPrivateKey, ArmakeError> {
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
        let (hash1, hash2, hash3) = super::generate_hashes(pbo, version, self.length);

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
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), ArmakeError> {
        output.write_cstring(&self.name)?;
        output.write_u32::<LittleEndian>(self.length / 16 * 9 + 20)?;
        output.write_all(b"\x07\x02\x00\x00\x00\x24\x00\x00")?;
        output.write_all(b"RSA2")?;
        output.write_u32::<LittleEndian>(self.length)?;
        output.write_u32::<LittleEndian>(self.exponent)?;
        output.write_bignum(&self.n, (self.length / 8) as usize)?;
        output.write_bignum(&self.p, (self.length / 16) as usize)?;
        output.write_bignum(&self.q, (self.length / 16) as usize)?;
        output.write_bignum(&self.dmp1, (self.length / 16) as usize)?;
        output.write_bignum(&self.dmq1, (self.length / 16) as usize)?;
        output.write_bignum(&self.iqmp, (self.length / 16) as usize)?;
        output.write_bignum(&self.d, (self.length / 8) as usize)?;
        Ok(())
    }
}

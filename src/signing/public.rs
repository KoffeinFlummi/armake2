use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use openssl::bn::{BigNum, BigNumContext};

use crate::error;
use crate::io::{ReadExt, WriteExt};
use crate::{ArmakeError, BISign, PBO};

pub struct BIPublicKey {
    pub name: String,
    pub length: u32,
    pub exponent: u32,
    pub n: BigNum,
}

impl BIPublicKey {
    /// Reads a public key from the given input.
    pub fn read<I: Read>(input: &mut I) -> Result<BIPublicKey, ArmakeError> {
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
    pub fn verify(&self, pbo: &PBO, signature: &BISign) -> Result<(), ArmakeError> {
        let (real_hash1, real_hash2, real_hash3) =
            super::generate_hashes(pbo, signature.version, self.length);

        let mut ctx = BigNumContext::new().unwrap();

        let exponent = BigNum::from_u32(self.exponent).unwrap();

        let mut signed_hash1: BigNum = BigNum::new().unwrap();
        signed_hash1
            .mod_exp(&signature.sig1, &exponent, &self.n, &mut ctx)
            .unwrap();
        let mut signed_hash2: BigNum = BigNum::new().unwrap();
        signed_hash2
            .mod_exp(&signature.sig2, &exponent, &self.n, &mut ctx)
            .unwrap();
        let mut signed_hash3: BigNum = BigNum::new().unwrap();
        signed_hash3
            .mod_exp(&signature.sig3, &exponent, &self.n, &mut ctx)
            .unwrap();

        if real_hash1 != signed_hash1 {
            let (s, r) = display_hashes(signed_hash1, real_hash1);
            return Err(error!(
                "Hash 1 doesn't match\nSigned hash: {}\nReal hash:   {}",
                s, r
            ));
        }

        if real_hash2 != signed_hash2 {
            let (s, r) = display_hashes(signed_hash2, real_hash2);
            return Err(error!(
                "Hash 2 doesn't match\nSigned hash: {}\nReal hash:   {}",
                s, r
            ));
        }

        if real_hash3 != signed_hash3 {
            let (s, r) = display_hashes(signed_hash3, real_hash3);
            return Err(error!(
                "Hash 3 doesn't match\nSigned hash: {}\nReal hash:   {}",
                s, r
            ));
        }

        Ok(())
    }

    /// Write public key to output.
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), ArmakeError> {
        output.write_cstring(&self.name)?;
        output.write_u32::<LittleEndian>(self.length / 8 + 20)?;
        output.write_all(b"\x06\x02\x00\x00\x00\x24\x00\x00")?;
        output.write_all(b"RSA1")?;
        output.write_u32::<LittleEndian>(self.length)?;
        output.write_u32::<LittleEndian>(self.exponent)?;
        output.write_bignum(&self.n, (self.length / 8) as usize)?;
        Ok(())
    }
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

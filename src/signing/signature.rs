use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use openssl::bn::BigNum;

use crate::{ArmakeError, BISignVersion};
use crate::error;
use crate::io::{ReadExt, WriteExt};

pub struct BISign {
    pub version: BISignVersion,
    pub name: String,
    pub length: u32,
    pub exponent: u32,
    pub n: BigNum,
    pub sig1: BigNum,
    pub sig2: BigNum,
    pub sig3: BigNum
}

impl BISign {
    /// Reads a signature from the given input.
    pub fn read<I: Read>(input: &mut I) -> Result<BISign, ArmakeError> {
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
    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), ArmakeError> {
        output.write_cstring(&self.name)?;
        output.write_u32::<LittleEndian>(self.length / 8 + 20)?;
        output.write_all(b"\x06\x02\x00\x00\x00\x24\x00\x00")?;
        output.write_all(b"RSA1")?;
        output.write_u32::<LittleEndian>(self.length)?;
        output.write_u32::<LittleEndian>(self.exponent)?;
        output.write_bignum(&self.n, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        output.write_bignum(&self.sig1, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(self.version.into())?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        output.write_bignum(&self.sig2, (self.length / 8) as usize)?;
        output.write_u32::<LittleEndian>(self.length / 8)?;
        output.write_bignum(&self.sig3, (self.length / 8) as usize)?;
        Ok(())
    }
}

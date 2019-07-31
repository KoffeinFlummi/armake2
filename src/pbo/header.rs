use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::ArmakeError;
use crate::io::{ReadExt, WriteExt};

pub struct PBOHeader {
    pub filename: String,
    pub packing_method: u32,
    pub original_size: u32,
    pub reserved: u32,
    pub timestamp: u32,
    pub data_size: u32,
}

#[derive(PartialEq)]
pub enum PackingMethod {
    Uncompressed,
    Packed,
    ProductEntry,
    Unknown,
}

impl PBOHeader {
    pub fn read<I: Read>(input: &mut I) -> Result<PBOHeader, ArmakeError> {
        Ok(PBOHeader {
            filename: input.read_cstring()?,
            packing_method: input.read_u32::<LittleEndian>()?,
            original_size: input.read_u32::<LittleEndian>()?,
            reserved: input.read_u32::<LittleEndian>()?,
            timestamp: input.read_u32::<LittleEndian>()?,
            data_size: input.read_u32::<LittleEndian>()?,
        })
    }

    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), ArmakeError> {
        output.write_cstring(&self.filename)?;
        output.write_u32::<LittleEndian>(self.packing_method)?;
        output.write_u32::<LittleEndian>(self.original_size)?;
        output.write_u32::<LittleEndian>(self.reserved)?;
        output.write_u32::<LittleEndian>(self.timestamp)?;
        output.write_u32::<LittleEndian>(self.data_size)?;
        Ok(())
    }

    pub fn method(&self) -> PackingMethod {
        match self.packing_method {
            0x0000_0000 => { PackingMethod::Uncompressed },
            0x0430_7273 => { PackingMethod::Packed },
            0x5665_7273 => { PackingMethod::ProductEntry },
            _ => { PackingMethod::Unknown },
        }
    }
}

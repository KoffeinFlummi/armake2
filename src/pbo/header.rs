use std::io::{Read, Write, Error};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::io::{ReadExt, WriteExt};

pub struct PBOHeader {
    pub filename: String,
    pub packing_method: u32,
    pub original_size: u32,
    pub reserved: u32,
    pub timestamp: u32,
    pub data_size: u32
}

impl PBOHeader {
    pub fn read<I: Read>(input: &mut I) -> Result<PBOHeader, Error> {
        Ok(PBOHeader {
            filename: input.read_cstring()?,
            packing_method: input.read_u32::<LittleEndian>()?,
            original_size: input.read_u32::<LittleEndian>()?,
            reserved: input.read_u32::<LittleEndian>()?,
            timestamp: input.read_u32::<LittleEndian>()?,
            data_size: input.read_u32::<LittleEndian>()?
        })
    }

    pub fn write<O: Write>(&self, output: &mut O) -> Result<(), Error> {
        output.write_cstring(&self.filename)?;
        output.write_u32::<LittleEndian>(self.packing_method)?;
        output.write_u32::<LittleEndian>(self.original_size)?;
        output.write_u32::<LittleEndian>(self.reserved)?;
        output.write_u32::<LittleEndian>(self.timestamp)?;
        output.write_u32::<LittleEndian>(self.data_size)?;
        Ok(())
    }
}

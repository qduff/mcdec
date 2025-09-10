use std::{
    fmt::Debug,
    io::{self, Read},
};

use crate::{
    constants::{CodeAddress, DataAddress},
    BinaryReader, SectionParser,
};

#[derive(Debug)]
pub struct PcToLineNumEntry {
    pub addr: CodeAddress,
    pub file: DataAddress,
    pub symbol: DataAddress,
    pub linenum: i32,
}

pub type PcToLineNumData = Vec<PcToLineNumEntry>;

pub struct PcToLineNumParser;

impl<R: Read> SectionParser<R> for PcToLineNumParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let n = reader.read_u16()?;
        let mut entries = Vec::new();

        for _i in 0..n {
            entries.push(PcToLineNumEntry {
                addr: CodeAddress::new_from_local(reader.read_i32()? as u32), // really should assert type here
                file: DataAddress::new_from_local(reader.read_i32()? as u32),
                symbol: DataAddress::new_from_local(reader.read_i32()? as u32),
                linenum: reader.read_i32()?,
            });
        }
        Ok(super::SectionKind::PcToLineNum(entries))
    }
}

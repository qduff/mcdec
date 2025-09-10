use std::io::{self, Read};

use crate::{BinaryReader, SectionParser};

pub type ExceptionsData = Vec<TableEntry>;

pub struct ExceptionParser; // TODO

impl<'a, R: Read> BinaryReader<'a, R> {
    fn read_u24(&mut self) -> io::Result<u64> {
        Ok((self.read_u8()? as u64) << 16
            | (self.read_u8()? as u64) << 8
            | (self.read_u8()? as u64))
    }
}

#[derive(Debug)]
pub struct TableEntry {
    pub try_begin: u64,
    pub try_end: u64,
    pub handle_begin: u64,
}

fn read_table_entry<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<TableEntry> {
    Ok(TableEntry {
        try_begin: reader.read_u24()?,
        try_end: reader.read_u24()?,
        handle_begin: reader.read_u24()?,
    })
}

impl<R: Read> SectionParser<R> for ExceptionParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let n = reader.read_u16().unwrap();
        Ok(super::SectionKind::Exceptions(
            (0..n)
                .map(|_| read_table_entry(reader))
                .collect::<io::Result<Vec<TableEntry>>>()?,
        ))
    }
}

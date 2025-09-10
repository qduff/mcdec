use std::{
    fmt::Debug,
    io::{self, Read},
};

use crate::{BinaryReader, SectionParser};

pub type UUID = [u8; 20];

#[derive(Debug)]

pub struct UUIDParser;

impl<R: Read> SectionParser<R> for UUIDParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let bytes = reader.read_bytes::<20>()?;

        // more modern devices (fenix 8/enduro 3/etc) may still have 5 more bytes (zeros)
        if reader.get_remaining() == 5 {
            reader.consume(5)?; 
        } else if reader.get_remaining() != 0 {
            panic!("invalid UUID section!")
        }

        Ok(super::SectionKind::UUID(bytes))
    }
}

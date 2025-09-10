use std::{
    fmt::Debug,
    io::{self, Read},
};

use crate::{BinaryReader, SectionParser};

/// Holds the developer signature data
#[derive(Debug)]
pub struct DevSigData {
    pub sig1: [u8; 512],         // sha1
    pub sig2: Option<[u8; 512]>, // sha512
    pub modulus: [u8; 512],
    pub exponent: i32,
}
pub struct DevSigParser;

impl<R: Read> SectionParser<R> for DevSigParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        Ok(super::SectionKind::DevSig(Box::new(DevSigData {
            sig1: reader.read_bytes::<512>()?,
            modulus: reader.read_bytes::<512>()?,
            exponent: reader.read_i32()?,
            sig2: if reader.get_remaining() >= 512 {
                Some(reader.read_bytes::<512>()?)
            } else {
                None
            },
        })))
    }
}

use std::{
    fmt::{self, Debug},
    io::{self, Read},
};

use crate::{BinaryReader, SectionParser};

#[derive(Debug)]
pub struct CIQVersion(u8, u8, u8);

impl fmt::Display for CIQVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

#[derive(Debug)]
pub struct GlanceOffsets {
    pub data: i32,
    pub code: i32,
}

#[derive(Debug)]
pub struct Flags {
    pub glance_support: bool,
    pub profiling_enabled: bool,
    pub sensor_pairing_support: bool,
}

#[derive(Debug)]
pub struct HeaderData {
    pub header_version: u8,
    pub ciqver: CIQVersion,
    pub background_offsets: Option<GlanceOffsets>,
    pub apptrial: Option<bool>,
    pub glance_offsets: Option<GlanceOffsets>,
    pub flags: Option<Flags>,
}

pub struct HeaderParser;

impl<R: Read> SectionParser<R> for HeaderParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        Ok(super::SectionKind::Header(HeaderData {
            header_version: reader.read_u8()?,
            ciqver: CIQVersion(reader.read_u8()?, reader.read_u8()?, reader.read_u8()?),
            background_offsets: match reader.has_remaining() {
                true => Some(GlanceOffsets {
                    data: reader.read_i32()?,
                    code: reader.read_i32()?,
                }),
                false => None,
            },
            apptrial: match reader.has_remaining() {
                true => Some(reader.read_u8()? != 0),
                false => None,
            },
            glance_offsets: match reader.has_remaining() {
                true => {
                    let _ = reader.consume(8);
                    Some(GlanceOffsets {
                        data: reader.read_i32()?,
                        code: reader.read_i32()?,
                    })
                }
                false => None,
            },
            flags: match reader.has_remaining() {
                true => {
                    let v = reader.read_u32()?;
                    Some(Flags {
                        glance_support: v & 1 != 0,
                        profiling_enabled: v & 2 != 0,
                        sensor_pairing_support: v & 4 != 0,
                    })
                }
                false => None,
            },
        }))
    }
}

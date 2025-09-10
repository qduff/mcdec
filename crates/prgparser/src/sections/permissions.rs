use std::{
    fmt::Debug,
    io::{self, Read},
};

use crate::{BinaryReader, SectionParser};

#[derive(Debug)]
pub struct PermissionsData {
    pub perms: Vec<i32>,
}

pub struct PermissionsParser;

impl<R: Read> SectionParser<R> for PermissionsParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let n = reader.read_u16()?;
        let mut perms = Vec::new();

        for _i in 0..n {
            perms.push(reader.read_i32()?);
        }
        Ok(super::SectionKind::Permissions(PermissionsData { perms }))
    }
}

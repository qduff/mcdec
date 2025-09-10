/// AKA LinkTable
use std::{
    fmt::Debug,
    io::{self, Read},
};

use crate::{ BinaryReader, SectionParser};


#[derive(Debug)]
pub struct Link{
    pub module: i32,
    pub link: i32,
}

#[derive(Debug)]
pub struct ImportData {
    pub links: Vec<Link>,
}
pub struct ImportParser;

impl<R: Read> SectionParser<R> for ImportParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let mut links = Vec::new();
        for _i in 0..reader.read_u16()? {
            links.push(Link { module: reader.read_i32()?, link: reader.read_i32()? });
        }
        Ok(super::SectionKind::Import(ImportData {
            links
        }))
    }
}

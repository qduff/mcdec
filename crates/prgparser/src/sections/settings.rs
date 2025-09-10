use std::{
    fmt::Debug,
    io::{self, Read},
};

use crate::{BinaryReader, SectionParser};

#[derive(Debug)]
pub struct SettingsData {}

pub struct SettingsParser; // TODO

impl<R: Read> SectionParser<R> for SettingsParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        reader.consume(reader.get_remaining())?;
        Ok(super::SectionKind::Settings(SettingsData {}))
    }
}

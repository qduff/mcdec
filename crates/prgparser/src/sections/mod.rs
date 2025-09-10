use std::{
    fmt::Debug,
    io::{self, Read},
};

use crate::{BinaryReader, SectionParser};

pub mod code;
pub mod data;
pub mod devsig;
pub mod entrypoints;
pub mod exceptions;
pub mod header;
pub mod imports;
pub mod pctolinenum;
pub mod permissions;
pub mod resources;
pub mod settings;
pub mod symbols;
pub mod uuid;

#[derive(Debug)]
pub enum SectionKind {
        UUID(uuid::UUID),
        Header(header::HeaderData),
        EntryPoints(entrypoints::EntryPointsData),
        Code(code::CodeData),
        Data(data::DataData),
        DevSig(Box<devsig::DevSigData>),
        Permissions(permissions::PermissionsData),
        Import(imports::ImportData),
        PcToLineNum(pctolinenum::PcToLineNumData),
        Symbols(symbols::SymbolData),
        Settings(settings::SettingsData),
        Resources(resources::ResourceData),
        Exceptions(exceptions::ExceptionsData),
        UnknownSection(),
}

#[derive(Debug)]
pub struct Section {
    pub address: u64,
    pub length: i32,
    pub kind: SectionKind,
}

pub struct UnkownSectionParser;

impl<R: Read> SectionParser<R> for UnkownSectionParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<SectionKind> {
        reader.consume(reader.get_remaining())?; // todo remove
        Ok(SectionKind::UnknownSection())
    }
}


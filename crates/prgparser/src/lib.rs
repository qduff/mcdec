use std::{
    fmt::Debug,
    io::{self, Read},
};

pub mod addressed_container;
mod binary_reader;
pub mod constants;
pub mod opcodes;
pub mod sections;

use constants::SectionMagic;
use num_enum::TryFromPrimitive;
use sections::*;

pub use {binary_reader::BinaryReader, sections::SectionKind};

trait SectionParser<R: Read> {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<sections::SectionKind>;
}

pub struct Parser<'a, R: Read> {
    binaryreader: BinaryReader<'a, R>,
}

impl<'a, R: Read> Parser<'a, R> {
    pub fn new(reader: BinaryReader<'a, R>) -> Self {
        Self {
            binaryreader: reader,
        }
    }

    pub fn parse(mut self) -> io::Result<ProgramSections> {
        let mut sections = Vec::new();
        while self.binaryreader.get_remaining() >= 8 {
            let magic_val = self.binaryreader.read_u32().unwrap();
            let length = self.binaryreader.read_i32().unwrap();
            let address = self.binaryreader.get_local_position();

            let magic =
                SectionMagic::try_from_primitive(magic_val).unwrap_or(SectionMagic::Unknown);

            let parser: &dyn SectionParser<R> = match &magic {
                SectionMagic::UUID => &uuid::UUIDParser,
                SectionMagic::Header | SectionMagic::HeaderVersioned => &header::HeaderParser,
                SectionMagic::EntryPoints => &entrypoints::EntrypointParser,
                SectionMagic::Data => &data::DataParser,
                SectionMagic::Code => &code::CodeParser,
                SectionMagic::DeveloperSignatureBlock => &devsig::DevSigParser,
                SectionMagic::Permissions => &permissions::PermissionsParser,
                SectionMagic::ClassImport => &imports::ImportParser,
                SectionMagic::PcToLineNum => &pctolinenum::PcToLineNumParser,
                SectionMagic::Symbols => &symbols::SymbolParser,
                SectionMagic::Settings => &settings::SettingsParser,
                SectionMagic::ResourceBlock => &resources::ResourceParser,
                SectionMagic::Exceptions => &exceptions::ExceptionParser,
                unknown_magic => {
                    // panic!(
                    //     "Cannot parse {unknown_magic:?} (magic: {magic_val:?}) @{} L:{length})",
                    //     self.binaryreader.get_local_position()
                    // );
                    &UnkownSectionParser
                }
            };

            sections.push(Section {
                address,
                length,
                kind: {
                    let mut reader = self.binaryreader.mut_slice(length as u64);
                    let parsed = parser.parse(&mut reader)?;
                    let rem = reader.get_remaining();
                    assert_eq!(rem, 0, "{rem} bytes remaining in {magic:?} section!");
                    parsed
                },
            });
        }
        // dbg!(&sections);
        Ok(ProgramSections(sections))
    }
}

#[derive(Debug)]
pub struct ProgramSections(std::vec::Vec<Section>);

macro_rules! implement_section_accessors {
    ( $( $get_fn:ident, $take_fn:ident : $variant:ident => $inner_type:ty ),* ) => {
        $(
            pub fn $get_fn(&self) -> Option<&$inner_type> {
                self.0.iter().find_map(|section| {
                    if let SectionKind::$variant(data) = &section.kind {
                        Some(data)
                    } else {
                        None
                    }
                })
            }

            pub fn $take_fn(&mut self) -> Option<$inner_type> {
                let index = self.0.iter().position(|section| {
                    matches!(section.kind, SectionKind::$variant(_))
                });

                if let Some(i) = index {
                    let section = self.0.remove(i);
                    if let SectionKind::$variant(data) = section.kind {
                        Some(data)
                    } else {
                        unreachable!()
                    }
                } else {
                    None
                }
            }
        )*
    };
}

impl ProgramSections {
    implement_section_accessors!(
        get_uuid_section,        take_uuid_section:        UUID        => uuid::UUID,
        get_header_section,      take_header_section:      Header      => header::HeaderData,
        get_entry_points_section,take_entry_points_section:EntryPoints => entrypoints::EntryPointsData,
        get_code_section,        take_code_section:        Code        => code::CodeData,
        get_data_section,        take_data_section:        Data        => data::DataData,
        get_dev_sig_section,     take_dev_sig_section:     DevSig      => Box<devsig::DevSigData>,
        get_permissions_section, take_permissions_section: Permissions => permissions::PermissionsData,
        get_import_section,      take_import_section:      Import      => imports::ImportData,
        get_pc_to_line_num_section, take_pc_to_line_num_section: PcToLineNum => pctolinenum::PcToLineNumData,
        get_symbols_section,     take_symbols_section:     Symbols     => symbols::SymbolData,
        get_settings_section,    take_settings_section:    Settings    => settings::SettingsData,
        get_resources_section,   take_resources_section:   Resources   => resources::ResourceData,
        get_exceptions_section,  take_exceptions_section:  Exceptions  => exceptions::ExceptionsData
    );
}

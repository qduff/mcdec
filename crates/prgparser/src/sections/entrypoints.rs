use std::{
    fmt::Debug,
    io::{self, Read},
};

// use crate::SizedBinaryReader;

use crate::{constants::SymbolAddress, sections::entrypoints, BinaryReader, SectionParser};

#[derive(Debug)]
pub struct EntryPoint {
    id: [u8; 16],
    module_id: SymbolAddress,
    class_id: SymbolAddress,
    label_id: SymbolAddress,
    iconlabel_id: i32,
    flags: i32,
}

//todo flags enum or whatever(and otehrs)

#[derive(Debug)]
pub struct EntryPointsData {
    pub entrypoints: Vec<EntryPoint>,
}

pub struct EntrypointParser;

impl<R: Read> SectionParser<R> for EntrypointParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let count = reader.read_u16()?;
        let mut entrypoints = Vec::new();
        // dbg!(count);
        for _i in 0..count {
            let id: [u8; 16] = reader.read_bytes::<16>()?;
            let module_id = SymbolAddress::new_from_local(reader.read_u32()?);
            let class_id = SymbolAddress::new_from_local(reader.read_u32()?);
            let label_id = SymbolAddress::new_from_local(reader.read_u32()?); // appname
            let iconlabel_id = reader.read_i32()?;
            let flags = reader.read_i32()?;
            entrypoints.push(EntryPoint {
                id,
                module_id,
                class_id,
                label_id,
                iconlabel_id,
                flags,
            });
        }
        // dbg!(&entrypoints);
        Ok(super::SectionKind::EntryPoints(EntryPointsData {
            entrypoints,
        }))
    }
}

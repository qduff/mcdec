use std::{
    collections::HashMap,
    io::{self, Read},
};

use crate::{BinaryReader, SectionParser};

pub type SymbolData = HashMap<u32, String>;

pub struct SymbolParser;

impl<R: Read> SectionParser<R> for SymbolParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let symbol_count = reader.read_u16()? as usize;

        let mut offset_to_symbol_id = HashMap::with_capacity(symbol_count);

        for _ in 0..symbol_count {
            let symbol = reader.read_u32()?;
            let string_offset = reader.read_u32()?;
            offset_to_symbol_id.insert(string_offset, symbol);
        }

        let mut symbols = HashMap::with_capacity(symbol_count);

        while reader.has_remaining() {
            let pos = reader.get_local_position() as u32;
            reader.read_u8()?;
            
            let length = reader.read_u16()?;
            let string = reader.new_string(length as u64)?;

            if let Some(symbol_id) = offset_to_symbol_id.remove(&pos) {
                symbols.insert(symbol_id, string);
            } else {
                panic!("Malformed SymbolSection: Found a string at an unexpected offset: {pos}")
            }

            reader.read_u8()?; // null terminator
        }
        // dbg!(   symboldata.iter().for_each(|f| eprint!("{:x} {}\n", f.0, f.1)));
        Ok(super::SectionKind::Symbols(symbols))
    }
}

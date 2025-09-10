use std::io::{self, Read};

use crate::{
    addressed_container::AddressedContainer,
    opcodes::{get_args, Opcode},
    BinaryReader, SectionParser,
};

pub type CodeData = AddressedContainer<Opcode>;

pub struct CodeParser;

impl<R: Read> SectionParser<R> for CodeParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let opcodes = Opcode::parse_stream(reader, get_args).unwrap();
        Ok(super::SectionKind::Code(opcodes))
    }
}

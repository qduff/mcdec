use core::panic;
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{self, Read},
};

use crate::{
    constants::{ApiNativeAddress, CodeAddress, DataAddress, SymbolAddress},
    BinaryReader, SectionParser,
};

#[derive(Debug, PartialEq)]

pub enum ClassTypes {
    Data(DataAddress),
    ApiNative(ApiNativeAddress),
}

#[repr(u32)]
#[derive(Debug, PartialEq)]
pub enum FieldValue {
    Null = 0,
    Int(i32) = 1, // AKA Number
    Float(f32) = 2,
    String(DataAddress) = 3,
    Object = 4,
    Array(DataAddress) = 5,
    Method(CodeAddress) = 6,
    Class(ClassTypes) = 7, // apinative or dataaddress
    Symbol = 8,
    Boolean(bool) = 9,
    Module(DataAddress) = 10, // todo Which
    Dictionary(DataAddress) = 11,
    Resource = 12,
    PrimitiveObject = 13,
    Long = 14,
    Double = 15,
    WeakRef = 16,
    PrimitiveModule = 17,
    SystemPointer = 18,
    Char = 19,
    Bytearray(DataAddress) = 20,
    SystemData = 21,
    ResourceID = 22,
}

#[derive(Debug)]
pub struct Flags(u8);

impl std::fmt::Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 != 0 {
            f.write_str("(")?;
            if self.0 & 0b001 == 0b001 {
                f.write_str("C")?;
            }
            if self.0 & 0b010 == 0b010 {
                f.write_str("H")?;
            }
            if self.0 & 0b100 == 0b100 {
                f.write_str("S")?;
            }
            f.write_str(")")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Field {
    pub symbol: SymbolAddress,
    pub value: FieldValue,
    pub flags: Flags,
}

fn parse_field<R: Read>(reader: &mut BinaryReader<R>, varsized: bool) -> io::Result<Field> {
    let key = reader.read_u32()?;

    let value_type = if varsized {
        reader.read_u8()?
    } else {
        (key & 15) as u8
    };

    let raw_value = reader.read_u32()?;

    let symbol = (key >> 8) & 0xFFFFFF;
    let flags = ((key >> 4) & 0xF) as u8;

    let value = match value_type {
        0 => FieldValue::Null,
        1 => FieldValue::Int(raw_value as i32),
        2 => FieldValue::Float(f32::from_bits(raw_value)),
        3 => FieldValue::String(DataAddress::new_from_global(raw_value).unwrap()),
        5 => FieldValue::Array(DataAddress::new_from_global(raw_value).unwrap()), // TODO VERIFY!!
        6 => FieldValue::Method(
            CodeAddress::new_from_global(raw_value)
                .unwrap_or_else(|| panic!("RAWVAL: {:#x}", raw_value)),
        ),

        7 => FieldValue::Class({
            DataAddress::new_from_global(raw_value)
                .map(ClassTypes::Data)
                .or_else(|| ApiNativeAddress::new_from_global(raw_value).map(ClassTypes::ApiNative))
                .expect("Fatal: Neither x() nor y() could produce a value.")
        }), // can be apinative too!!!
        // 7 =>  panic!(),
        9 => FieldValue::Boolean(raw_value != 0),
        11 => FieldValue::Dictionary(DataAddress::new_from_global(raw_value).unwrap()),

        10 => {
            if flags != 0 {
                // bruh
                FieldValue::Null
            } else {
                // println!("{}", raw_value);
                FieldValue::Module(DataAddress::new_from_global(raw_value).unwrap())
                // FieldValue::Module(get_section_offset(raw_value))
                // panic!("SHOULD BE MOUL")
            }
        }
        20 => FieldValue::Bytearray(DataAddress::new_from_global(raw_value).unwrap()),
        x => todo!("unhandled value_type {} (raw_value:{})", x, raw_value),
    };

    Ok(Field {
        symbol: SymbolAddress::new_from_local(symbol),
        flags: Flags(flags),
        value,
    })
}

#[derive(Debug)]
pub struct Class {
    pub extends_offset: Option<DataAddress>,
    pub statics: u32,
    pub parent_module_id: Option<SymbolAddress>,
    pub module_id: Option<SymbolAddress>,
    pub app_type: u16,
    pub fields: Vec<Field>,
}

// CIQ 5.0.0 (f7 onwards?) seems to use variable sized class definitions.
fn parse_class<R: Read>(reader: &mut BinaryReader<R>, varsized: bool) -> io::Result<Class> {
    let flags = if !varsized {
        None
    } else {
        Some(reader.read_u8()?)
    };

    // can extend apinativesection!!!
    let extends_offset = if !varsized || flags.unwrap() & 0b0001 == 0b0001 {
        let raw_extends_offset = reader.read_u32()?;
        if raw_extends_offset == 0 {
            None
        } else {
            DataAddress::new_from_global(raw_extends_offset)
        }
    } else {
        None
    };

    let statics = if !varsized || flags.unwrap() & 0b0010 == 0b0010 {
        reader.read_u32()?
    } else {
        0
    };

    let parent_module_id = if !varsized || flags.unwrap() & 0b0100 == 0b0100 {
        let pid = reader.read_u32()?;
        if pid != 0 {
            Some(SymbolAddress::new_from_local(pid))
        } else {
            None
        }
    } else {
        None
    };

    let module_id = if !varsized || flags.unwrap() & 0b1000 == 0b1000 {
        let mid = reader.read_u32()?;
        if mid != 0 {
            Some(SymbolAddress::new_from_local(mid))
        } else {
            None
        }
    } else {
        None
    };

    let app_type = reader.read_u16()?;

    let n = match varsized {
        true => reader.read_u16()?,
        false => reader.read_u8()? as u16,
    };

    let fields = (0..n)
        .map(|_| parse_field(reader, varsized))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Class {
        extends_offset,
        statics,
        parent_module_id,
        module_id,
        app_type,
        fields,
    })
}

#[derive(Debug)]
pub struct Array {
    items: Vec<FieldValue>,
}

#[derive(Debug)]
pub enum DataEntryTypes {
    String(String),
    Class(Class),
    Array(Vec<FieldValue>),
    ByteArray(Vec<u8>),
    Dictionary(Vec<(FieldValue, FieldValue)>),
}

pub type DataData = HashMap<DataAddress, DataEntryTypes>;

pub struct DataParser;

fn ParseContainerField<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<FieldValue> {
    Ok(match reader.read_u8()? {
        0 => FieldValue::Null,
        1 => FieldValue::Int(reader.read_i32()?),
        3 => FieldValue::String({ DataAddress::new_from_global(reader.read_u32()?).unwrap() }),
        x => todo!("field {x}"),
    })
}

fn parse_entry<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<DataEntryTypes> {
    let first = reader.read_u8()?;
    if first == 1 {
        // TODO spliit into functions!!!!
        // String Type
        let length = reader.read_u16()?;
        let string = reader.new_string(length as u64).unwrap();
        reader.read_u8()?;
        Ok(DataEntryTypes::String(string))
    } else if first == 3 {
        // JSON type (array, bytearray, dict) - this is CONTAINER
        let total_len = reader.read_u32()?;

        let magic = reader.read_u32()?;
        if magic == 0xABCDABCD {
            let blocklen = reader.read_i32()? as u64;
            // dbg!(blocklen, reader.get_remaining());
            let mut block_reader = reader.mut_slice(blocklen);
            // dbg!("sliced!");
            while block_reader.has_remaining() {
                let length = block_reader.read_u16()?;
                let string = block_reader.new_string(length as u64).unwrap();
                // dbg!(string);
                // reader.read_u8()?;

                // DataEntryTypes::String(string)
            }
            drop(block_reader);
            reader.consume(4)?;

            // let strings = HashMap::new();
            // reader.consume(4)?;
        }

        reader.consume(4)?;

        let container_magic = reader.read_u8()?;
        let container_len = reader.read_u32()?;

        let mut sized_reader = reader.mut_slice(reader.get_remaining());

        let ret = match container_magic {
            20 => Ok(DataEntryTypes::ByteArray(
                sized_reader.read_n_bytes(sized_reader.get_remaining())?,
            )),
            5 => {
                let mut arr: Vec<FieldValue> = Vec::new();
                for _ in 0..container_len {
                    arr.push(ParseContainerField(&mut sized_reader)?);
                }
                assert!(container_len == arr.len() as u32);
                Ok(DataEntryTypes::Array(arr))
            }
            11 => {
                let mut dict = Vec::new();
                for _ in 0..container_len {
                    dict.push((
                        ParseContainerField(&mut sized_reader)?,
                        ParseContainerField(&mut sized_reader)?,
                    ));
                }
                assert!(container_len == dict.len() as u32);
                Ok(DataEntryTypes::Dictionary(dict))
            }
            _ => panic!("Unkown type"),
        };

        drop(sized_reader);
        reader.consume(1)?;
        // dbg!(&ret);

        ret
    } else if reader.read_bytes::<3>()? == [0xa5, 0x5d, 0xef] && (first == 0xc2 || first == 0xc1) {
        // Class
        Ok(DataEntryTypes::Class(parse_class(reader, first == 0xc2)?))
    } else {
        panic!("invalid magic")
    }
}

impl<R: Read> SectionParser<R> for DataParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let mut entries = HashMap::new();
        while reader.has_remaining() {
            let addr = reader.get_local_position();
            let entry = parse_entry(reader)?;
            entries.insert(DataAddress::new_from_local(addr as u32), entry);
        }
        Ok(super::SectionKind::Data(entries))
    }
}

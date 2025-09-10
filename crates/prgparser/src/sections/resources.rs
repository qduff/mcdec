use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    io::{self, Read},
};

use num_enum::TryFromPrimitive;

use crate::{BinaryReader, SectionParser};

#[derive(Debug)]
pub struct ResourceData {
    // pub perms: Vec<i32>,
    langs: HashMap<u32, HashMap<u32, String>>,
}

pub struct ResourceParser;

#[repr(u32)]
#[derive(Debug, TryFromPrimitive, Hash, Eq, PartialEq, Clone, Copy)]
pub enum JumpTableType {
    STRINGS = 0x8000A2,
    DRAWABLES = 0x8000A3,
    FONTS = 0x8000A4,
    JSONDATA = 0x8005C8,
    BARRELS = 0x80072C,
}

#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum ResourceType {
    BITMAP = 0,
    STRING = 1,
    FONT = 2,
    // MENU = -1,
    // DRAWABLE = -1,
    // LAYOUT = -1,
    // PROP = -1,
    // SETTING = -1,
    // SETTING_GRP = -1,
    JSON = 3,
    ANIMATION = 4,
}

// fn parse_symbol_table<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<HashMap<u32, u32>>{
//     Ok(symbol_table)

// }

impl<R: Read> SectionParser<R> for ResourceParser {
    fn parse(&self, reader: &mut BinaryReader<R>) -> io::Result<super::SectionKind> {
        let n = reader.read_u16()?;
        let mut overview_table = BTreeMap::new();
        for _ in 0..n {
            let jtt_magic = reader.read_u32()?;
            let string_offset = reader.read_u32()?;

            let magic = JumpTableType::try_from_primitive(jtt_magic)
                .expect("Resource: Invalid JumpTableType");

            overview_table.insert(string_offset, magic);
        }

        let mut subtable = BTreeMap::new();

        for (addr, magic) in overview_table {
            let diff = addr as i32 - reader.get_local_position() as i32;
            if diff > 0 {
                reader.consume(diff as u64)?
            } else if diff < 0 {
                panic!("missed!")
            }

            let n = reader.read_u16()?;
            for _ in 0..n {
                let symbol = reader.read_u32()?;
                let offset = reader.read_u32()?;
                subtable.insert(offset, (magic, symbol));
            }
            if magic == JumpTableType::STRINGS {
                // reader.consume(10)?;
            }
        }

        // let mut keys: Vec<u32> = subtable.keys().cloned().collect();
        // keys.sort();
        // for key in keys {
        //     println!("{}: {:?}", key, subtable[&key]);
        // }

        let mut langs = HashMap::new();

        for (addr, (jtt, id)) in subtable.iter() {
            if *jtt == JumpTableType::BARRELS {
                continue; // TODO this is not correct!!!!
            }
            let diff = *addr as i32 - reader.get_local_position() as i32;
            if diff > 0 {
                reader.consume(diff as u64)?
            } else if diff < 0 {
                panic!("missed!")
            }

            match jtt {
                JumpTableType::DRAWABLES => {
                    let val = reader.read_u8()?;
                    if val == ResourceType::BITMAP as u8 || val == ResourceType::ANIMATION as u8 {
                        // todo split out
                        match reader.read_i32()? {
                            0xC11EE5E => {
                                // BITMAP TODO
                                let len = reader.read_i32()?;
                                reader.consume(len as u64)?;
                            }
                            0x2001600D => {
                                // ANIMATION TODO
                                let header = reader.read_bytes::<26>()?;
                                // little endian???
                                let _ = reader.consume(20)?;
                                let video_length = reader.read_i32()?;
                                let video_content = reader.read_n_bytes(video_length as u64)?;
                            }
                            469762076 => {
                                panic!("todo advimage");
                                // advanced image format todo
                                let len = reader.read_u16()?;
                                let width = reader.read_u16()?;
                                let height = reader.read_u16()?;
                                let format = reader.read_u16()?; // default, yuv png jpg todo 0-3
                                let alpha_bits = reader.read_u8()? & 0b111;

                                let caa_len = if alpha_bits >0 {
                                    Some(reader.read_u32()?)
                                } else{
                                    None
                                };

                                let subheader_length = reader.read_u16()?;
                                let subheader = if subheader_length > 0 {
                                    Some(reader.read_n_bytes(subheader_length as u64))
                                } else {
                                    None
                                };

                                if caa_len.is_some() && caa_len.unwrap() > 0 {
                                    reader.consume(caa_len.unwrap() as u64)?; // todo tidy
                                }
                            }
                            other => panic!("invalid drawable magic {:#x}", other),
                        }
                    }
                    // dbg!(val);
                }
                JumpTableType::STRINGS => {
                    let string_count = reader.read_u16()?;
                    let mut symbols = HashMap::with_capacity(string_count as usize);

                    for _ in 0..string_count {
                        let symbol = reader.read_u32()?;
                        let string_offset = reader.read_u32()?;
                        symbols.insert(string_offset, symbol);
                    }

                    let mut strings = HashMap::new();

                    for _ in 0..string_count {
                        let pos = reader.get_local_position() as u32;
                        if let Some(symbol_id) = symbols.remove(&pos) {
                            assert!(reader.read_u8()? == ResourceType::STRING as u8);
                            let length = reader.read_u16()?;
                            let string = reader.new_string(length as u64)?;
                            strings.insert(symbol_id, string);
                            reader.consume(1)?; //nullterm
                        } else {
                            panic!("Malformed SymbolSection: Found a string at an unexpected offset: {pos}")
                        }
                    }
                    langs.insert(*id, strings);
                }
                JumpTableType::FONTS => {
                    assert!(reader.read_u8()? == ResourceType::FONT as u8);
                    match reader.read_i32()? {
                        61511 => {
                            // normal font
                            let height = reader.read_i32()?;
                            let glyph_count = reader.read_i32()?;
                            let min = reader.read_i32()?;
                            let data_size = reader.read_i32()?;

                            const BYTES_PER_NONUNICODE_GLYPH_TABLE: u8 = 3;

                            let glyph_buffer = reader.read_n_bytes(
                                BYTES_PER_NONUNICODE_GLYPH_TABLE as u64 * glyph_count as u64,
                            );

                            let glyph_sentinel = reader.read_i32()?;

                            let extra_data = reader.read_n_bytes(data_size as u64);
                        }
                        62011 => {
                            // unicode
                            let header = reader.read_bytes::<14>()?;
                            let glyph_count = reader.read_i32()?;
                            let raw_data_size = reader.read_u32()?;
                            let cmap_header = reader.read_bytes::<12>()?;
                            let cmap_groups = reader.read_i32()?;
                            let cmap_table_buffer = reader.read_n_bytes(12 * cmap_groups as u64)?;
                            let glyph_table_buffer = reader.read_n_bytes(4 * glyph_count as u64)?;
                            let data_size = if reader.read_i32()? == -855638003 {
                                reader.read_u32()?
                            } else {
                                raw_data_size
                            };
                            let extra_data = reader.read_n_bytes(data_size as u64)?;
                        }
                        _ => panic!("invalid font type"),
                    }
                }
                JumpTableType::JSONDATA => {
                    assert!(reader.read_u8()? == ResourceType::JSON as u8);
                    let len = reader.read_i32()?;
                    let json = reader.read_n_bytes(len as u64);
                    // dbg!(json);
                }
                JumpTableType::BARRELS => todo!(),
            }
        }

        // dbg!(&langs);
        // remaining is language table

        // dbg!(reader.get_remaining());
        // TODO data remaining : JSON-esque???

        reader.consume(reader.get_remaining())?;

        // panic!("end!!");
        Ok(super::SectionKind::Resources(ResourceData { langs }))
    }
}

use crate::addressed_container::AddressedContainer;
use crate::addressed_container::SparseMap;
use crate::constants::{CodeAddress, DataAddress, SymbolAddress};
use crate::BinaryReader;
use core::fmt;
use std::fmt::Debug;
use std::io::{self, Read};
use mcd_traits::DisplayWithResolver;
use mcd_traits::TInstruction;
use mcd_traits::{display_with_resolver, AddressResolver};


macro_rules! define_opcodes_and_parser {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $EnumName:ident {
            $(
                $(#[$variant_meta:meta])*
                $Variant:ident $( ($($ArgTy:ty),+ $(,)?) )? = $Value:expr
            ),* $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        $vis enum $EnumName {
            $(
                $(#[$variant_meta])*
                $Variant $( ($($ArgTy),+) )?,
            )*
        }



        impl $EnumName {
            pub fn parse_stream<R>(
                mut reader: &mut BinaryReader<R>,
                argfn: fn(iter: &mut BinaryReader<R>, opcode: Opcode) -> io::Result<Opcode>
            ) -> io::Result<AddressedContainer<Opcode>>
            where
                R: Read,
            {
                let mut opcodes = Vec::new();
                let mut addr_map = Vec::new();

                while let Ok(byte) = reader.read_u8() {
                    let curpos = reader.get_local_position() - 1;
                    let instruction = match byte {
                        $(
                            $Value => {
                                // stringify!($($($ArgTy)+)?);
                                define_opcodes_and_parser!(@parse_args $EnumName::$Variant, argfn, reader, $($($ArgTy),+)?)
                            }
                        ),*
                        invalid_byte => Err(Error::new(ErrorKind::Other, format!("Read invalid byte{}", invalid_byte))),

                    }?;
                    opcodes.push(instruction);
                    addr_map.push(curpos as usize);
                }
                Ok(AddressedContainer::new(opcodes,  SparseMap::new_presorted(addr_map)))
            }
        }
    };

    (@parse_args $VariantPath:path, $argfn:expr,  $reader:ident, ) => {
        Ok($VariantPath)
    };

    (@parse_args $VariantPath:path, $argfn:expr, $reader:ident, $( $TY:ty ),* ) => {
        $argfn(&mut $reader, $VariantPath( $( <$TY>::default() ),* ))
    };
}

define_opcodes_and_parser! {
    #[repr(i32)]
    #[derive( PartialEq, Clone, Copy, Debug)]
    pub enum Opcode {
        /// No Opcode
        NOP = 0,
        /// Reserve n local variables
        INCSP(i32) = 1,
        /// Pop top of stack and discard
        POPV = 2,
        /// Add top elements of stack
        ADD = 3,
        /// Subtract top elements of stack
        SUB = 4,
        /// Multiply top elements of stack
        MUL = 5,
        /// Divide top elements of stack
        DIV = 6,
        /// Bitwise AND top elements of stack
        AND = 7,
        /// Bitwise OR top elements of stack
        OR = 8,
        /// Bitwise modulo top elements of stack
        MOD = 9,
        /// Shift left
        SHL = 10,
        /// Arithmetic shift right
        SHR = 11,
        /// Bitwise XOR top elements of stack
        XOR = 12,
        /// Pop stack, deref symbol from addr, and store
        /// [..., s] -> [..., *s]
        /// retrieve an attribute from a module???
        GETV = 13,
        /// Store top of stack into symbol of second to top
        /// [..., s, v] -> *s = v -> [...]
        PUTV = 14,
        /// Invoke function with n args. The n arguments are passed on the stack, with the function itself below.
        INVOKEM(u8) = 15,
        ///  index second to top of stack with top of stack, and store on stack
        AGETV = 16,
        /// store top of stack in array third to top indexed by second to top (top of stack returns to array - consumes two items)
        APUTV = 17,
        /// Push local n onto stack
        LGETV(u8) = 18,
        /// Pop stack and store in local n
        LPUTV(u8) = 19,
        /// New array of length given by item on top of stack
        NEWA = 20,
        /// Initialize new class given symbol on top of stack
        NEWC = 21,
        /// Return from function
        RETURN = 22,
        /// Return from JSR call (used for try statement finally)
        RET = 23,
        /// New string // TODO SymbolAddress typwe? or DataAddress???
        NEWS(DataAddress) = 24,
        /// Jump to speficied address
        GOTO(CodeAddress) = 25,
        /// Compare equality of top two variables on stack
        EQ = 26,
        LT = 27,
        LTE = 28,
        GT = 29,
        GTE = 30,
        NE = 31,
        ISNULL = 32,
        /// 'instanceof' infix operation
        ISA = 33,
        /// 'has' infix operation
        CANHAZPLZ = 34,
        /// Jump to codeaddress that lies within current function - used for try statement finally - see RET
        JSR(CodeAddress) = 35,
        /// Not sure but is implemented as NOP (legacy??)
        TS = 36,
        /// Push int
        IPUSH(i32) = 37,
        /// Push float
        FPUSH(f32) = 38,
        /// Push symbol
        SPUSH(SymbolAddress) = 39,
        /// If value on top of stack is true (!0?) jump to specified address
        BT(CodeAddress) = 40,
        /// If value on top of stack is false (0?) jump to specified address
        BF(CodeAddress) = 41,
        /// Push Method
        FRPUSH = 42,
        BPUSH(i32) = 43,
        NPUSH = 44,
        /// Complement?
        INV = 45,
        DUP(u8) = 46,
        NEWD = 47,
        /// Get module based on symbol id at top of stack
        GETM = 48,
        LPUSH(u64) = 49,
        DPUSH(f64) = 50,
        THROW = 51,
        CPUSH(char) = 52,
        /// Number of arguments, including context which is always passed
        ARGC(usize) = 53,
        NEWBA = 54,
        IPUSHZ = 55,
        /// Push immediate 1 byte
        IPUSH1(i8) = 56,
        /// Push immediate 2 bytes
        IPUSH2(i16) = 57,
        /// Push immediate 3 bytes
        IPUSH3(i32) = 58, // actually i24!
        /// Push a float 0 onto stack
        FPUSHZ = 59,
        /// Push a long 0 onto stack (ref to heap alloced)
        LPUSHZ = 60,
        /// Push a double 0 onto stack (ref to heap alloced)
        DPUSHZ = 61,
        /// Push a boolean true onto the stack
        BTPUSH = 62,
        /// Push a boolean false onto the stack
        BFPUSH = 63,
        APUSH(DataAddress) = 64,
        BAPUSH(DataAddress) = 65,
        HPUSH(DataAddress) = 66,
        GETSELFV(SymbolAddress) = 67,
        GETSELF = 68,
        GETMV(SymbolAddress, SymbolAddress) = 69,
        GETLOCALV(u8, SymbolAddress) = 70,
        GETSV(SymbolAddress) = 71,
        /// (V2) Invoke with zero args. Essentially a combination of FRPUSH, INVOKE
        INVOKEMZ = 72,
        APUTVDUP = 73,
        ARGCINCSP(usize, u8) = 74,
        ISNOTNULL = 75,
    }
}
use std::io::{Error, ErrorKind};
pub fn get_args<R: Read>(reader: &mut BinaryReader<R>, opcode: Opcode) -> io::Result<Opcode> {
    Ok(match opcode {
        Opcode::IPUSH(_) => Opcode::IPUSH(reader.read_i32()?),
        Opcode::FPUSH(_) => Opcode::FPUSH(reader.read_f32()?),
        Opcode::SPUSH(_) => Opcode::SPUSH(SymbolAddress::new_from_local(reader.read_u32()?)), // SYMBOL_SECTION
        Opcode::CPUSH(_) => Opcode::CPUSH(reader.read_u32()? as u8 as char), // TODO 1 or 4 bytes???
        Opcode::GETSELFV(_) => Opcode::GETSELFV(SymbolAddress::new_from_local(reader.read_u32()?)),
        Opcode::GETSV(_) => Opcode::GETSV(SymbolAddress::new_from_local(reader.read_u32()?)),

        Opcode::LGETV(_) => Opcode::LGETV(reader.read_u8()?),
        Opcode::LPUTV(_) => Opcode::LPUTV(reader.read_u8()?),
        Opcode::INVOKEM(_) => Opcode::INVOKEM(reader.read_u8()?),
        Opcode::ARGC(_) => Opcode::ARGC(reader.read_u8()? as usize),
        Opcode::INCSP(_) => Opcode::INCSP(reader.read_u8()? as i32),
        Opcode::BPUSH(_) => Opcode::BPUSH(reader.read_u8()? as i32),
        Opcode::DUP(_) => Opcode::DUP(reader.read_u8()?),

        Opcode::BT(_) => Opcode::BT(CodeAddress::new_from_local(
            (reader.read_i16()? as i32 + reader.get_local_position() as i32) as u32,
        )), // nb selection of types, should work in all cases
        Opcode::BF(_) => {
            Opcode::BF(CodeAddress::new_from_local((reader.read_i16()? as i32 + reader.get_local_position() as i32) as u32))
        }
        Opcode::GOTO(_) => Opcode::GOTO(CodeAddress::new_from_local(
            (reader.read_i16()? as i32 + reader.get_local_position() as i32) as u32,
        )),
        Opcode::JSR(_) => {
            Opcode::JSR(CodeAddress::new_from_local((reader.read_i16()? as i32 + reader.get_local_position() as i32) as u32))
        }

        Opcode::NEWS(_) => Opcode::NEWS(DataAddress::new_from_local(reader.read_u32()?)),

        // V2 Opcodes i think
        Opcode::ARGCINCSP(_, _) => Opcode::ARGCINCSP(reader.read_u8()? as usize, reader.read_u8()?),
        Opcode::GETLOCALV(_, _) => Opcode::GETLOCALV(reader.read_u8()?, SymbolAddress::new_from_local(reader.read_u32()?)),

        Opcode::IPUSH1(_) => Opcode::IPUSH1(reader.read_i8()?),
        Opcode::IPUSH2(_) => Opcode::IPUSH2(reader.read_i16()?),
        Opcode::IPUSH3(_) => Opcode::IPUSH3(
            // big endian decoding
            (reader.read_i8()? as i32) << 16
                | (reader.read_u8()? as i32) << 8
                | reader.read_u8()? as i32,
        ),
        Opcode::HPUSH(_) => Opcode::HPUSH(DataAddress::new_from_local(reader.read_u32()?) ),
        Opcode::BAPUSH(_) => Opcode::BAPUSH(DataAddress::new_from_local(reader.read_u32()?) ),
        Opcode::APUSH(_) => Opcode::APUSH(DataAddress::new_from_local(reader.read_u32()?) ),

        Opcode::GETMV(_, _) => Opcode::GETMV(SymbolAddress::new_from_local(reader.read_u32()?), SymbolAddress::new_from_local(reader.read_u32()?)),
        
        Opcode::DPUSH(_) => Opcode::DPUSH(reader.read_f64()?),
        Opcode::LPUSH(_) => Opcode::LPUSH(reader.read_u64()?),

        x => todo!("get_args for {:?} @ {}", x, reader.get_local_position()),
    })
}



impl DisplayWithResolver for Opcode {
    fn fmt_with_resolver<R: AddressResolver>(&self, f: &mut fmt::Formatter<'_>, resolver: &R) -> fmt::Result {
        match self {
            Opcode::SPUSH(symbol) => write!(f, "SPUSH {}", display_with_resolver(symbol, resolver)),
            Opcode::NEWS(data) => write!(f, "NEWS {}", display_with_resolver(data, resolver)),
            Opcode::GETSELFV(symbol) => write!(f, "GETSELFV {}", display_with_resolver(symbol, resolver)),
            Opcode::GETMV(s1, s2) => write!(f, "GETMV {}, {}", display_with_resolver(s1, resolver), display_with_resolver(s2, resolver)),
            Opcode::GETLOCALV(idx, symbol) => write!(f, "GETLOCALV {}, {}", idx, display_with_resolver(symbol, resolver)),
            Opcode::GETSV(symbol) => write!(f, "GETSV {}", display_with_resolver(symbol, resolver)),
            Opcode::APUSH(addr) => write!(f, "APUSH {}", display_with_resolver(addr, resolver)),
            Opcode::BAPUSH(addr) => write!(f, "BAPUSH {}", display_with_resolver(addr, resolver)),
            Opcode::HPUSH(addr) => write!(f, "HPUSH {}", display_with_resolver(addr, resolver)),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl TInstruction for Opcode{
    
}

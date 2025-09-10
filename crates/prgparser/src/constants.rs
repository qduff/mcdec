use num_enum::TryFromPrimitive;
use std::ops::Add;
use std::ops::Sub;
use std::fmt;
use mcd_traits::{AddressResolver, DisplayWithResolver};

#[repr(u32)]
#[derive(Debug, TryFromPrimitive)]
pub enum SectionMagic {
    UUID = 0x0000001D,
    Header = 0xD000D000,
    // If resourceId supported a.k.a 5.0.0 or greater
    HeaderVersioned = 0xD000D00D,
    EntryPoints = 0x6060C0DE,
    Permissions = 0x6000DB01,
    Data = 0xDA7ABABE,
    Code = 0xC0DEBABE,
    ExtendedCode = 0xC0DE10AD,
    Debug = 0xD0000D1E,
    PcToLineNum = 0xC0DE7AB1,
    ClassImport = 0xC1A557B1,
    Exceptions = 0x0ECE7105,
    ExtendedExceptions = 0xEECE7105,
    Symbols = 0x5717B015,
    Settings = 0x5E771465,
    AppUnlock = 0xD011AAA5,
    ResourceBlock = 0xF00D600D,
    BackgroundResourceBlock = 0xDEFECA7E,
    GlanceResourceBlock = 0xD00DFACE,
    AppStoreSignatureBlock = 0x00005161,
    DeveloperSignatureBlock = 0xE1C0DE12,
    StringResourceSymbols = 0xBAADA555,
    Complications = 0xFACEDA7A,
    WatchfaceConfig = 0xFACE00FF,
    NativeLibrary = 0xE1FFB10B,
    Unknown,
}

const REGION_SIZE: Addr = 0x1000_0000;

type Addr = u32;

pub trait LocalAddress: GlobalAddress {
    fn get_local_address(&self) -> Addr;
}

pub trait GlobalAddress {
    fn get_global_address(&self) -> Addr;
}


macro_rules! define_address_type {
    ($struct_name:ident, $offset:expr) => {
        #[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
        pub struct $struct_name(Addr);

        impl GlobalAddress for $struct_name {
            fn get_global_address(&self) -> Addr {
                self.0 + $offset
            }
        }

        impl LocalAddress for $struct_name {
            fn get_local_address(&self) -> Addr {
                self.0
            }
        }

        impl Default for $struct_name {
            fn default() -> $struct_name {
                $struct_name(0)
            }
        }
        // impl Display for $struct_name {
        //     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //         write!(f, "TODODISPLAY {}", self.0)
        //     }
        // }

        impl Add for $struct_name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                $struct_name(self.0 + rhs.0)
            }
        }

        impl Sub for $struct_name {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                $struct_name(self.0 - rhs.0)
            }
        }

        impl $struct_name {
            pub fn new_from_global(global_addr: Addr) -> Option<Self> {
                if global_addr > $offset + REGION_SIZE {
                    None
                }else{
                    Some(Self(global_addr - $offset))
                }
            }

            pub fn new_from_local(local_addr: Addr) -> Self {
                Self(local_addr)
            }

            pub fn value(&self) -> Addr {
                self.0
            }
        }
    };
}

define_address_type!(DataAddress, 0x0000_0000);
define_address_type!(CodeAddress, 0x1000_0000);
define_address_type!(ApiDataAddress, 0x2000_0000);
define_address_type!(ApiCodeAddress, 0x3000_0000);
define_address_type!(ApiNativeAddress, 0x4000_0000);
define_address_type!(ExtendedCodeAddress, 0x5000_0000);
define_address_type!(NativeAddress, 0x6000_0000);
define_address_type!(SymbolAddress, 0x8000_0000);

impl DisplayWithResolver for SymbolAddress {
    fn fmt_with_resolver<R: AddressResolver>(&self, f: &mut fmt::Formatter<'_>, resolver: &R) -> fmt::Result {
        match resolver.resolve_symbol(self.0) {
            Some(name) => f.write_str(name),
            None => write!(f, "sym_{:}", self.0),
        }
    }
}

impl DisplayWithResolver for DataAddress {
    fn fmt_with_resolver<R: AddressResolver>(&self, f: &mut fmt::Formatter<'_>, resolver: &R) -> fmt::Result {
        match resolver.resolve_data(self.0) {
            Some(name) => write!(f, "\"{}\"", name.escape_default()),
            None => write!(f, "data_{:X}", self.0),
        }
    }
}

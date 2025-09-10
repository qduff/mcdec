use std::path::PathBuf;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    fmt::{self, Debug, Display},
    fs::{self, File},
    io::BufRead,
};

use mcd_traits::AddressResolver;
use prgparser::{
    addressed_container::AddressedContainer,
    constants::{DataAddress, LocalAddress, SymbolAddress},
    opcodes::Opcode,
    sections::{
        data::{ClassTypes, DataData, DataEntryTypes, FieldValue},
        symbols::SymbolData,
    },
    ProgramSections,
};
use std::io::BufReader;

use crate::{
    disassembler::{DisassemblyError, DisassemblyFunction},
    sourcedata::SourceData,
    ssa::{SSAError, SSAFunction},
};
pub mod disassembler;
pub mod ssa;

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorUnion {
    Disassembler(DisassemblyError),
    SSA(SSAError),
}

#[derive(Debug)]
pub enum AnalysisError<F> {
    Failure(F),
    DependencyFailure(ErrorUnion),
}

/// Represents the status of an IL.
#[derive(Default)]
enum OptionalIL<T, E> {
    #[default]
    NotAnalyzed,
    Some(T),
    Failure(E),
}

impl<T, E> OptionalIL<T, E> {
    fn unwrap(&self) -> &T {
        match self {
            OptionalIL::NotAnalyzed | OptionalIL::Failure(_) => panic!("unwrap on OptionalIL"),
            OptionalIL::Some(x) => x,
        }
    }
}

impl<T, E> Display for OptionalIL<T, E>
where
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionalIL::NotAnalyzed => write!(f, "Not Analyzed"),
            OptionalIL::Some(_) => write!(f, "OK"),
            OptionalIL::Failure(x) => write!(f, "Failed: {x:?}"),
        }
    }
}

// todo change with_ getters to use these enums instead
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ILType {
    Disassembly,
    SSA,
}

impl ILType {
    pub fn all() -> impl Iterator<Item = Self> {
        [ILType::Disassembly, ILType::SSA].iter().copied()
    }
}

impl Display for ILType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

mod sourcedata;

pub struct Function {
    name: Vec<SymbolAddress>,
    arg_count: usize,
    container: AddressedContainer<Opcode>,
    sourcedata: sourcedata::SourceData,
    // ILs
    disassembly: OptionalIL<DisassemblyFunction, DisassemblyError>,
    ssa: OptionalIL<SSAFunction, SSAError>,
}

impl Function {
    fn new(
        name: Vec<SymbolAddress>,
        arg_count: usize,
        container: AddressedContainer<Opcode>,
    ) -> Self {
        Function {
            name,
            arg_count,
            container,
            sourcedata: SourceData::default(),
            disassembly: OptionalIL::default(),
            ssa: OptionalIL::default(),
        }
    }

    pub fn get_start_address(&self) -> Option<usize> {
        self.container.start_addr()
    }

    pub fn get_name(&self) -> &Vec<SymbolAddress> {
        &self.name
    }

    pub fn get_arg_count(&self) -> usize {
        self.arg_count
    }

    pub fn get_source_data(&self) -> &SourceData {
        &self.sourcedata
    }

    pub fn get_il_status(&self, il_type: ILType) -> String {
        match il_type {
            ILType::Disassembly => format!("{}", self.disassembly),
            ILType::SSA => format!("{}", self.ssa),
        }
    }

    fn ensure_disassembly_analyzed(&mut self) {
        if let OptionalIL::NotAnalyzed = &self.disassembly {
            let result = DisassemblyFunction::disassemble(&self.container);
            self.disassembly = match result {
                Ok(dis) => OptionalIL::Some(dis),
                Err(e) => OptionalIL::Failure(e),
            };
        }
    }

    pub fn with_disassembly<F, R>(&mut self, f: F) -> Result<R, AnalysisError<DisassemblyError>>
    where
        F: FnOnce(&DisassemblyFunction) -> R,
    {
        self.ensure_disassembly_analyzed();
        match &self.disassembly {
            OptionalIL::Some(dis) => Ok(f(dis)),
            OptionalIL::Failure(reason) => Err(AnalysisError::Failure(reason.clone())),
            OptionalIL::NotAnalyzed => unreachable!(),
        }
    }

    fn ensure_ssa_analyzed(&mut self) {
        self.ensure_disassembly_analyzed();
        if let OptionalIL::Failure(_) = self.disassembly {
            return;
        }

        if let OptionalIL::NotAnalyzed = self.ssa {
            // todo impl Into from previous IL
            let result = ssa::perform_ssa_function(self.disassembly.unwrap());
            self.ssa = match result {
                Ok(dec) => OptionalIL::Some(dec),
                Err(msg) => OptionalIL::Failure(msg),
            };
        }
    }

    pub fn with_ssa<F, R>(&mut self, f: F) -> Result<R, AnalysisError<SSAError>>
    where
        F: FnOnce(&SSAFunction) -> R,
    {
        self.ensure_ssa_analyzed();

        if let OptionalIL::Failure(dis_err) = &self.disassembly {
            return Err(AnalysisError::DependencyFailure(ErrorUnion::Disassembler(
                dis_err.clone(),
            )));
        }

        match &self.ssa {
            OptionalIL::Some(dec) => Ok(f(dec)),
            OptionalIL::Failure(dec_err) => Err(AnalysisError::Failure(dec_err.clone())),
            OptionalIL::NotAnalyzed => unreachable!(),
        }
    }
}


fn class_recursive_function_find(
    data_section: &DataData,
    code_section: &prgparser::sections::code::CodeData,
) -> BTreeMap<usize, (usize, Vec<SymbolAddress>)> {
    let mut results = BTreeMap::new();

    let mut temp_traverse = Vec::new();
    class_recursive_function_find_inner(
        DataAddress::new_from_local(0),
        &mut results,
        &mut temp_traverse,
        data_section,
        code_section,
    );
    results
}

/// TODO also add non canonical instances (Extends = true)
/// TODO apparently there can also be other primary instances????
fn class_recursive_function_find_inner(
    id: DataAddress,
    results: &mut BTreeMap<usize, (usize, Vec<SymbolAddress>)>,
    ancestors: &mut Vec<SymbolAddress>,
    data_section: &DataData,
    code_section: &prgparser::sections::code::CodeData,
) {
    if let DataEntryTypes::Class(class) = data_section.get(&id).unwrap() {
        let extends = &class.extends_offset;

        for field in &class.fields {
            ancestors.push(field.symbol);

            match &field.value {
                FieldValue::Method(code_address) => {
                    let first = code_section
                        .item_at_address(code_address.get_local_address() as usize)
                        .unwrap();

                    let count = match first {
                        Opcode::ARGC(count) | Opcode::ARGCINCSP(count, _) => *count,
                        _ => 0,
                    };

                    results.insert(
                        code_address.get_local_address() as usize,
                        (count, ancestors.clone()),
                    );
                    // .is_some()
                    // .then(|| if code_address.get_local_address() != 0 {panic!("ALREADY EXISTS!!")});
                }

                FieldValue::Class(ClassTypes::Data(section_addr))
                | FieldValue::Module(section_addr) => {
                    // ignore non-canonical instances - TODO add all as instances
                    if extends.is_none_or(|e| e != *section_addr) {
                        class_recursive_function_find_inner(
                            *section_addr,
                            results,
                            ancestors,
                            data_section,
                            code_section,
                        );
                    }
                }
                _ => {}
            }
            ancestors.pop();
        }
    }
}

fn generate_functions(
    code_section: &prgparser::sections::code::CodeData,
    stubs: BTreeMap<usize, (usize, Vec<SymbolAddress>)>,
) -> Vec<Function> {
    let mut functions = Vec::new();

    let code_end_idx = code_section.len();

    let mut iter = stubs.into_iter().peekable(); // Create a peekable iterator

    while let Some((function_start, (arg_count, symbols))) = iter.next() {
        let start_idx = code_section.addr_to_idx(function_start).unwrap();
        let end_idx = match iter.peek() {
            Some(&(next_function_start, _)) => {
                code_section.addr_to_idx(next_function_start).unwrap()
            }
            None => code_end_idx,
        };
        let view = code_section.slice(start_idx..end_idx).unwrap();
        let function = Function::new(symbols, arg_count, view);
        functions.push(function);
    }

    functions
}

pub struct SymbolDB {
    user_symbols: HashMap<u32, String>,
    api_db: Option<HashMap<u32, String>>,
    symbol_section: Option<SymbolData>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum SymbolSource {
    ApiDb,
    SymbolSection,
    User,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Symbol<'a> {
    pub key: u32,
    pub name: &'a String,
    pub source: SymbolSource,
}

impl SymbolDB {
    fn new(symbol_section: Option<SymbolData>) -> Self {
        Self {
            api_db: SymbolDB::locate_symbol_db().and_then(SymbolDB::parse_symbol_db),
            user_symbols: HashMap::new(),
            symbol_section,
        }
    }

    pub fn set_symbol_name(&mut self, key: u32, name: String) {
        // TODO dont allow overwrite api_db, but do for symbol_section
        self.user_symbols.insert(key, name);
    }

    pub fn get_symbol_name(&self, key: u32) -> Option<&str> {
        self.api_db
            .as_ref()
            .and_then(|db| db.get(&key))
            .or_else(|| self.user_symbols.get(&key))
            .or_else(|| self.symbol_section.as_ref().and_then(|x| x.get(&key)))
            .map(|x| x.as_str())
    }

    /// Iterate over symbols, excluding duplicates
    pub fn iter_symbols(&self) -> impl Iterator<Item = Symbol<'_>> {
        let user_iter = self.user_symbols.iter().map(|(&key, name)| Symbol {
            key,
            name,
            source: SymbolSource::User,
        });

        let section_iter = self
            .symbol_section
            .iter()
            .flatten()
            .map(|(&key, name)| Symbol {
                key,
                name,
                source: SymbolSource::SymbolSection,
            });

        let api_iter = self
            .api_db
            .as_ref()
            .into_iter()
            .flatten()
            .map(|(&key, name)| Symbol {
                key,
                name,
                source: SymbolSource::ApiDb,
            });

            let mut chained = user_iter.chain(section_iter).chain(api_iter);
        let mut seen_keys = HashSet::new();

        std::iter::from_fn(move || {
            for symbol in chained.by_ref() {
                if seen_keys.insert(symbol.key) {
                    return Some(symbol);
                }
            }
            None
        })
    }
    #[cfg(target_os = "linux")]
    fn locate_symbol_db() -> Option<PathBuf> {
        let home_dir = env::var("HOME").ok()?;
        let vendor_path = PathBuf::from(home_dir).join(".Garmin/ConnectIQ/Sdks/");

        let entries = fs::read_dir(vendor_path).ok()?;

        let latest_sdk = entries
            .filter_map(|entry_result| {
                let entry = entry_result.ok()?;
                let path = entry.path();

                if !path.is_dir() {
                    return None;
                }

                let folder_name = entry.file_name().into_string().ok()?;

                folder_name
                    .strip_prefix("connectiq-sdk-lin-")
                    .and_then(|rem| rem.split('-').next())
                    .and_then(|ver| {
                        let parts: Vec<u32> =
                            ver.split('.').filter_map(|s| s.parse().ok()).collect();

                        if parts.len() == 3 {
                            Some(((parts[0], parts[1], parts[2]), path))
                        } else {
                            None
                        }
                    })
            })
            .max_by_key(|(version, _path)| *version)
            .map(|(_version, path)| path.join("bin/api.db"));

        latest_sdk
    }

    #[cfg(not(target_os = "linux"))]
    fn locate_symbol_db() -> Option<PathBuf> {
        None
    }

    fn parse_symbol_db(path: PathBuf) -> Option<HashMap<u32, String>> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        let mut map = HashMap::new();

        for line in reader.lines() {
            let line = line.ok()?;
            let mut parts = line.split_whitespace();
            if let (Some(symbol), Some(key_str)) = (parts.next(), parts.next()) {
                if parts.next().is_none() {
                    if let Ok(integer_key) = key_str.parse::<u32>() {
                        map.insert(integer_key, symbol.to_string());
                    }
                }
            }
        }
        Some(map)
    }
}

pub struct MCD {
    // raw_sections: ProgramSections,
    pub symbols: SymbolDB,
    pub functions: Vec<Function>,
}

impl MCD {
    pub fn new(sections: ProgramSections) -> MCD {
        let stubs = class_recursive_function_find(
            sections.get_data_section().unwrap(),
            sections.get_code_section().unwrap(),
        );
        let functions = generate_functions(sections.get_code_section().unwrap(), stubs);

        Self {
            // raw_sections: sections,
            functions,
            symbols: SymbolDB::new(sections.take_symbols_section()),
        }
    }
}

impl AddressResolver for SymbolDB{
    fn resolve_symbol(&self, addr: u32) -> Option<&str> {
        self.get_symbol_name(addr)
    }

    // todo spin out of here!
    fn resolve_data(&self, addr: u32) -> Option<&str> {
        None
    }
}
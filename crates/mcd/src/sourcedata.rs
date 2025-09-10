//! SourceData represents the

use core::fmt;

use prgparser::constants::DataAddress;

/// The PC2LN section (only in debug builds) gives mappings of instructions to 
/// the respective source code locations. If this section is present, we 
/// coalesce these for one function into this `SourceData`
#[derive(Default, Debug)]
pub struct SourceData {
    file: Option<DataAddress>, // TODO merge both to one option that must be updated together
    bounds: Option<(usize, usize)>,
}

impl SourceData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_file(&mut self, source_symbol: DataAddress) {
        self.file = Some(source_symbol)
    }

    pub fn exists(&self) -> bool {
        self.file.is_some()
    }

    pub fn add_line(&mut self, line: usize) {
        match self.bounds {
            None => {
                self.bounds = Some((line, line));
            }
            Some((current_min, current_max)) => {
                self.bounds = Some((
                    std::cmp::min(current_min, line),
                    std::cmp::max(current_max, line),
                ));
            }
        }
    }

    pub fn start(&self) -> Option<usize> {
        self.bounds.map(|(min, _)| min)
    }

    pub fn end(&self) -> Option<usize> {
        self.bounds.map(|(_, max)| max)
    }

    pub fn filename(&self) -> Option<DataAddress> {
        self.file
    }
}

impl fmt::Display for SourceData {
    // TODO make clear only for lines, not file
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.bounds, self.file) {
            (Some((min, max)), Some(_)) => write!(f, "{min}-{max}"),
            _ => write!(f, "[None]"),
        }
    }
}

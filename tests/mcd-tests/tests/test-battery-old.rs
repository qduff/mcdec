use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;
use std::panic;
use std::{fs, path::PathBuf};

use mcd::disassembler::DisassemblyError;
use prgparser::{BinaryReader, Parser};
use rstest::rstest;
use mcd::AnalysisError::Failure;

use mcd::MCD;


#[rstest]
fn test_single_prg_file(
    #[base_dir="${PRG_BASE_DIR:-/tmp/prgs}"]
    // #[base_dir="/home/qduff/Documents/ciq-decomp/apps-downloader/data"]
    #[files("**/*.prg")]
    #[mode = path]
    path: PathBuf,
) {
    // println!("Testing file: {}", path.display());

    let f = File::open(&path).unwrap();
    let len = f.metadata().unwrap().len();
    let mut buf_reader = BufReader::new(f);

    let binary_reader = BinaryReader::new(&mut buf_reader, len);
    let parsed = Parser::new(binary_reader).parse().unwrap();

    let mut base = MCD::new(parsed);

    for function in base.functions.iter_mut(){
        match function.with_disassembly(|disassembly| {}){
            Err(Failure(DisassemblyError::CannotDissasembleJSR)) => (),
            x =>  x.unwrap()
        }
    };

    
}

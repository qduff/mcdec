use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;
use std::panic;
use std::{fs, path::PathBuf};

use prgparser::{BinaryReader, Parser};
use rstest::rstest;

use mcd::MCD;

#[rstest]
fn test_single_prg_file(
    #[base_dir="${PRG_BASE_DIR:-/tmp/prgs}"] 
    #[files("**/*.prg")] 
    #[mode = path]
    path: PathBuf) {
    // println!("Testing file: {}", path.display());
    
    let f = File::open(&path).unwrap();
    let len = f.metadata().unwrap().len();
    let mut buf_reader = BufReader::new(f);

    let binary_reader = BinaryReader::new(&mut buf_reader, len);
    let parsed = Parser::new(binary_reader).parse().unwrap();

    let mut base = MCD::new(parsed);
    // let disassembled = base();
}


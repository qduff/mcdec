# PRG Parser

This crate provides a parser for the Garmin `.prg` file format, returning a struct of its constituent sections.

## Overview

The main entry point of this library is the `prgparser::Parser`. It takes a `BinaryReader` (a wrapper around a `Read` trait object) and produces a `ProgramSections` struct.

The `ProgramSections` struct contains all the sections found in the `.prg` file (e.g., code, data, resources, symbols) and provides accessor methods to retrieve them.

## Usage

Here is a basic example of how to use the parser:

```rust
use std::fs::File;
use std::io::BufReader;
use prgparser::{BinaryReader, Parser};

fn main() -> std::io::Result<()> {
    let file = File::open("path/to/your.prg")?;
    let len = file.metadata()?.len();
    let mut buf_reader = BufReader::new(file);
    let binary_reader = BinaryReader::new(&mut buf_reader, len);

    let mut parser = Parser::new(binary_reader);
    let program_sections = parser.parse()?;

    if let Some(code_section) = program_sections.get_code_section() {
        println!("Successfully parsed code section with {} instructions.", code_section.len());
    }

    if let Some(symbol_section) = program_sections.get_symbols_section() {
        println!("Found {} symbols.", symbol_section.len());
    }

    Ok(())
}
```

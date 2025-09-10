# Monkey C Decompiler (mcd)

This crate takes the parsed sections of a Monkey C program from the `prgparser` crate and performs the disassembly and decompilation.

## Intermediate Representations
The program is transformed into `Function` objects. Each function's code can then be transformed into various Intermediate Language (IL) representations on demand, currently there are:
*   **Disassembly:** A basic, human-readable representation of the bytecode.
*   **SSA (Static Single Assignment):** Local and stack state is simulated to generate SSA form.

## Usage

The `mcd` crate is primarily used by the `tui` crate to get the data to display. Here is a simplified example of its basic use:

```rust
use mcd::MCD;
use prgparser::{Parser, BinaryReader};
use std::fs::File;
use std::io::BufReader;

fn main() -> std::io::Result<()> {
    // 1. Parse the .prg file
    let file = File::open("path/to/your.prg")?;
    let len = file.metadata()?.len();
    let mut buf_reader = BufReader::new(file);
    let binary_reader = BinaryReader::new(&mut buf_reader, len);
    let program_sections = Parser::new(binary_reader).parse()?;

    // 2. Initialize the decompiler with the parsed sections
    let mut mcd = MCD::new(program_sections);

    // 3. Interact with the decompiled program
    println!("Found {} functions.", mcd.functions.len());

    if let Some(function) = mcd.functions.get_mut(0) {
        // Lazily generate and access the disassembly
        function.with_disassembly(|disassembly| {
            // ... use disassembly ...
        }).unwrap();
    }

    Ok(())
}
```

# Terminal User Interface (tui)

This crate provides an interactive terminal user interface for analyzing Monkey C programs. It is the primary front-end.

## Overview

The TUI is built using the [`ratatui`](https://ratatui.rs/) library and serves as a visual wrapper around the analysis performed by the `mcd` crate. It allows the user to navigate the decompiled program in a structured way.

## Usage

This crate provides the main executable. To run it, use the following command from the root of the workspace, providing the path to a `.prg` file:

```sh
cargo run --release -- <path/to/your.prg>
```

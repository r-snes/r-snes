use std::env;
mod rom;
use rom::{Rom, RomError};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
}

fn run() -> Result<(), RomError> {
    let args: Vec<String> = env::args().collect();

    let rom = Rom::load_from_file(&args[1])?;

    rom.print_rom_header();

    Ok(())
}

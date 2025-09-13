mod bus;
mod io;
mod memory_region;
mod rom;
mod wram;

use bus::Bus;
use rom::{Rom, RomError};
use std::{env, error::Error};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let bus = Bus::new(&args[1])?;

    bus.rom.print_rom_header();

    Ok(())
}

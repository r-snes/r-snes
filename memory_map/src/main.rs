use std::io;

mod rom;
use rom::{Rom, RomError, RomMapping};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
}

fn run() -> Result<(), RomError> {
    let mut rom = Rom::load_from_file("super_mario_world.smc")?;
    println!("ROM loaded successfully!");
    println!("ROM size: {} bytes", rom.size());

    rom.map = RomMapping::detect_rom_mapping(&rom.data);
    match rom.map {
        RomMapping::LoRom => println!("Detected: LoROM"),
        RomMapping::HiRom => println!("Detected: HiROM"),
        RomMapping::Unknown => println!("Detected: Unknown mapping"),
    }
    rom.print_rom_header();

    Ok(())
}

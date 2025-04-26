use std::io;

mod rom;
use rom::Rom;
use rom::RomMapping;
use rom::detect_rom_mapping;

fn main() -> io::Result<()> {
    // let rom = Rom::load_from_file("super_mario_world.smc")?;
    let rom = Rom::load_from_file("secret_of_mana.sfc")?;

    println!("ROM loaded successfully!");
    println!("ROM size: {} bytes", rom.size());

    // println!("First 16 bytes of ROM:");
    // for i in 0..16 {
    //     if let Some(byte) = rom.read_byte(i) {
    //         print!("{:02X} ", byte);
    //     }
    // }
    // println!();

    match detect_rom_mapping(&rom.data) {
        RomMapping::LoRom => println!("Detected: LoROM"),
        RomMapping::HiRom => println!("Detected: HiROM"),
        RomMapping::Unknown => println!("Detected: Unknown mapping"),
    }

    Ok(())
}

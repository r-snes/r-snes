use std::io;

mod rom;
use rom::Rom;

fn main() -> io::Result<()> {
    let rom = Rom::load_from_file("super_mario_world.smc")?;

    println!("ROM loaded successfully!");
    println!("ROM size: {} bytes", rom.size());

    println!("First 16 bytes of ROM:");
    for i in 0..16 {
        if let Some(byte) = rom.read_byte(i) {
            print!("{:02X} ", byte);
        }
    }
    println!();

    Ok(())
}

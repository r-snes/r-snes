use bus::bus::Bus;
use std::{env, error::Error};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let bus = Bus::new(&args[1])?;
    bus.rom.header.print_header_bytes();
    println!("{}", bus.rom.header);
    Ok(())
}

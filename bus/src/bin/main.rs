use apu::Apu;
use bus::bus::Bus;
use cpu::cpu::CPU;
use cpu::registers::Registers;
use ppu::ppu::PPU;
use std::cell::RefCell;
use std::rc::Rc;
use std::{env, error::Error};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let cpu = Rc::new(RefCell::new(CPU::new(Registers::default())));
    let ppu = Rc::new(RefCell::new(PPU::new()));
    let apu = Rc::new(RefCell::new(Apu::new()));
    let bus = Bus::new(&args[1], cpu, ppu, apu)?;
    bus.rom.print_rom_header();
    Ok(())
}

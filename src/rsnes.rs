use apu::Apu;
use bus::Bus;
use cpu::cpu::CPU;
use cpu::registers::Registers;
use ppu::ppu::PPU;

use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

pub struct Rsnes {
    pub rom_path: PathBuf,
    pub bus: Bus,
    pub cpu: CPU,
    pub ppu: PPU,
    pub apu: Apu,
    pub master_cycles: u64,
}

impl Rsnes {
    pub fn load_rom<P: AsRef<Path>>(rom_path: &P) -> Result<Self, Box<dyn Error>> {
        let bus = Bus::new(rom_path)?;
        let cpu = CPU::new(Registers::default());
        let ppu = PPU::new();
        let apu = Apu::new();

        Ok(Self {
            rom_path: rom_path.as_ref().to_path_buf().clone(),
            bus,
            cpu,
            ppu,
            apu,
            master_cycles: 0,
        })
    }

    pub fn update() {
        // I don't know what this function should do for now but
        // I guess this will be where we'll call CPU opcodes, read from memory, etc...
    }
}

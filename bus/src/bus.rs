use crate::memory_region::MemoryRegion;
use crate::rom::Rom;
use crate::wram::Wram;
use std::error::Error;
use std::path::Path;

pub struct Bus {
    pub wram: Wram,
    pub rom: Rom,
    // TODO : Add other peripherals here later
}

impl Bus {
    pub fn new<P: AsRef<Path>>(rom_path: P) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            wram: Wram::new(),
            rom: Rom::load_from_file(rom_path)?,
        })
    }

    pub fn read(&self, addr: u32) -> u8 {
        let bank = (addr >> 16) as u8;
        let offset = addr & 0xFFFF;

        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if offset < 0x2000 {
                    self.wram.read(addr & 0x1FFFF)
                } else if offset >= 0x8000 {
                    self.rom.read(addr)
                } else {
                    0xFF // TODO : Add other memory regions
                }
            }
            0x7E..=0x7F => self.wram.read(addr),
            0x40..=0x7D | 0xC0..=0xFF => self.rom.read(addr),
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        let bank = (addr >> 16) as u8;
        let offset = addr & 0xFFFF;

        match bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if offset < 0x2000 {
                    self.wram.write((addr & 0x1FFFF), value);
                } else {
                    // TODO : Add other memory regions
                }
            }
            0x7E..=0x7F => self.wram.write(addr, value),
            _ => {}
        }
    }
}

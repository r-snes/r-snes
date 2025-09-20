use crate::io::Io;
use crate::memory_region::MemoryRegion;
use crate::rom::Rom;
use crate::wram::Wram;
use common::snes_address::SnesAddress;
use std::error::Error;
use std::path::Path;

pub struct Bus {
    pub wram: Wram,
    pub rom: Rom,
    pub io: Io,
}

impl Bus {
    pub fn new<P: AsRef<Path>>(rom_path: P) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            rom: Rom::load_from_file(rom_path)?,
            wram: Wram::new(),
            io: Io::new(),
        })
    }

    #[allow(dead_code)]
    pub fn read(&self, addr: SnesAddress) -> u8 {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if addr.addr < 0x2000 {
                    self.wram.read(addr)
                } else if addr.addr >= 0x2000 && addr.addr < 0x6000 {
                    self.io.read(addr)
                } else if addr.addr >= 0x6000 && addr.addr < 0x8000 {
                    self.rom.read(addr) // TODO : Expansion port
                } else if addr.addr >= 0x8000 {
                    self.rom.read(addr)
                } else {
                    0xFF // TODO : Shouldn't come here, maybe just add debug ?
                }
            }
            0x7E..=0x7F => self.wram.read(addr),
            0x40..=0x7D | 0xC0..=0xFF => self.rom.read(addr),
        }
    }

    #[allow(dead_code)]
    pub fn write(&mut self, addr: SnesAddress, value: u8) {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF => {
                if addr.addr < 0x2000 {
                    self.wram.write(addr, value);
                } else if addr.addr >= 0x2000 && addr.addr < 0x6000 {
                    self.io.write(addr, value);
                } else if addr.addr >= 0x6000 && addr.addr < 0x8000 {
                    self.rom.write(addr, value); // TODO : Expansion port
                } else if addr.addr >= 0x8000 {
                    self.rom.write(addr, value); // ROM no writes handled in `rom`
                } else {
                    // TODO : Shouldn't come here, maybe just add debug ?
                }
            }
            0x7E..=0x7F => self.wram.write(addr, value),
            0x40..=0x7D | 0xC0..=0xFF => self.rom.write(addr, value),
        }
    }
}

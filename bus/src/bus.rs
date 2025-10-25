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

    pub fn read(&self, addr: SnesAddress) -> u8 {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF => match addr.addr {
                0x0000..0x2000 => self.wram.read(addr),
                0x2000..0x6000 => self.io.read(addr),
                0x6000..0x8000 => self.rom.read(addr), // TODO : Expansion port
                0x8000..=0xFFFF => self.rom.read(addr),
            },
            0x7E..=0x7F => self.wram.read(addr),
            0x40..=0x7D | 0xC0..=0xFF => self.rom.read(addr),
        }
    }

    pub fn write(&mut self, addr: SnesAddress, value: u8) {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF => match addr.addr {
                0x0000..0x2000 => self.wram.write(addr, value),
                0x2000..0x6000 => self.io.write(addr, value),
                0x6000..0x8000 => self.rom.write(addr, value), // TODO : Expansion port
                0x8000..=0xFFFF => self.rom.write(addr, value), // ROM no writes handled in `rom`
            },
            0x7E..=0x7F => self.wram.write(addr, value),
            0x40..=0x7D | 0xC0..=0xFF => self.rom.write(addr, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::test_rom::*;

    #[test]
    fn test_wram_read_write_through_bus() {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        let addr = SnesAddress {
            bank: 0x00,
            addr: 0x0010,
        };
        bus.write(addr, 0x42);
        assert_eq!(bus.read(addr), 0x42);

        let addr_mirror = SnesAddress {
            bank: 0x80,
            addr: 0x0010,
        };
        assert_eq!(bus.read(addr), 0x42);
        assert_eq!(bus.read(addr_mirror), 0x42);

        let real_addr = SnesAddress {
            bank: 0x7E,
            addr: 0x0010,
        };
        assert_eq!(bus.read(real_addr), 0x42);

        bus.write(real_addr, 0x21);
        assert_eq!(bus.read(real_addr), 0x21);
        assert_eq!(bus.read(addr), 0x21);
        assert_eq!(bus.read(addr_mirror), 0x21);
    }

    #[test]
    fn test_io_read_write_through_bus() {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        let addr = SnesAddress {
            bank: 0x00,
            addr: 0x2345,
        };
        bus.write(addr, 0x77);
        assert_eq!(bus.read(addr), 0x77);

        let addr_mirror = SnesAddress {
            bank: 0x9E,
            addr: 0x2345,
        };
        assert_eq!(bus.read(addr), 0x77);
        assert_eq!(bus.read(addr_mirror), 0x77);
    }

    #[test]
    fn test_rom_read_write_through_bus() {
        let mut rom_data = create_valid_lorom(0x100000 * 0x40);
        rom_data[0x0001] = 0x42;
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let mut bus = Bus::new(&rom_path).unwrap();

        let addr = SnesAddress {
            bank: 0x00,
            addr: 0x8001,
        };
        assert_eq!(bus.read(addr), 0x42);
        bus.write(addr, 0x21);
        assert_eq!(bus.read(addr), 0x42);

        let other_addr = SnesAddress {
            bank: 0x40,
            addr: 0x8001,
        };
        assert_eq!(bus.read(other_addr), 0);
        bus.write(other_addr, 0x21);
        assert_eq!(bus.read(other_addr), 0);
    }

    #[test]
    #[should_panic(expected = "ERROR: Couldn't extract value from ROM")]
    fn test_rom_read_out_of_range_panics() {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        let bus = Bus::new(&rom_path).unwrap();

        // Create an address mapped to an offset beyond the 128 KiB dummy ROM.
        let addr = SnesAddress {
            bank: 0x7D,
            addr: 0xFFFF,
        };
        bus.rom.read(addr);
    }
}

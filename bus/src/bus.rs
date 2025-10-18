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
                match addr.addr {
                    0x0000..0x2000 => self.wram.read(addr),
                    0x2000..0x6000 => self.io.read(addr),
                    0x6000..0x8000 => self.rom.read(addr), // TODO : Expansion port
                    0x8000..=0xFFFF => self.rom.read(addr),
                }
            }
            0x7E..=0x7F => self.wram.read(addr),
            0x40..=0x7D | 0xC0..=0xFF => self.rom.read(addr),
        }
    }

    #[allow(dead_code)]
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
    use std::io::Write;
    use tempfile::tempdir;

    /// Helper: create a temporary 128 KiB ROM file filled with 0x00
    fn create_temp_rom(vec: Vec<u8>) -> std::path::PathBuf {
        let dir = tempdir().unwrap();
        let rom_path = dir.path().join("test_rom.sfc");
        let mut f = std::fs::File::create(&rom_path).unwrap();

        f.write_all(&vec).unwrap();

        // Prevent the temp directory from being deleted before tests finish
        std::mem::forget(dir);
        rom_path
    }

    fn create_valid_lorom(size: usize) -> Vec<u8> {
        assert!(size >= 0x8000, "ROM must be at least 32KiB");
        let mut rom = vec![0u8; size];

        // --- Minimal internal header (LoROM) ---
        let base = 0x7FC0;
        // Title: 21 bytes padded with spaces
        let title = b"TEST ROM            "; // exactly 21 bytes
        rom[base..base + title.len()].copy_from_slice(title);

        // Map mode: LoROM
        rom[0x7FD5] = 0x20;
        // Cartridge type: plain
        rom[0x7FD6] = 0x00;
        // ROM size: 0x08 => 256 KiB (2^8 * 2 KiB)
        // use log2(size/2KiB)
        let rom_size_code = (size as f64 / 2048.0).log2().round() as u8;
        rom[0x7FD7] = rom_size_code;
        // SRAM size
        rom[0x7FD8] = 0x00;
        // Country
        rom[0x7FD9] = 0x01;
        // License
        rom[0x7FDA] = 0x33;
        // Version
        rom[0x7FDB] = 0x00;

        // Compute checksum and complement
        let sum: u16 = rom
            .iter()
            .enumerate()
            .filter(|(i, _)| !matches!(*i, 0x7FDC | 0x7FDD | 0x7FDE | 0x7FDF))
            .map(|(_, &b)| b as u32)
            .sum::<u32>() as u16;
        let checksum = sum;
        let complement = !checksum;

        rom[0x7FDC] = (complement & 0xFF) as u8;
        rom[0x7FDD] = (complement >> 8) as u8;
        rom[0x7FDE] = (checksum & 0xFF) as u8;
        rom[0x7FDF] = (checksum >> 8) as u8;

        rom
    }

    #[test]
    fn test_wram_read_write_through_bus() {
        let rom_data = create_valid_lorom(0x20000);
        let rom_path = create_temp_rom(rom_data);
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
        let rom_path = create_temp_rom(rom_data);
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
        let rom_path = create_temp_rom(rom_data);
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
        let rom_path = create_temp_rom(rom_data);
        let bus = Bus::new(&rom_path).unwrap();

        // Create an address mapped to an offset beyond the 128 KiB dummy ROM.
        let addr = SnesAddress {
            bank: 0x7D,
            addr: 0xFFFF,
        };
        bus.rom.read(addr);
    }
}

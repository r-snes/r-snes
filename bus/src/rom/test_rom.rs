//! Module which contains utility functions for
//! writing unit tests needing ROM objects

use crate::constants::{
    HEADER_RAM_SIZE_OFFSET, HEADER_ROM_HARDWARE_OFFSET, HEADER_SIZE, HIROM_BANK_SIZE,
    HIROM_HEADER_OFFSET, LOROM_BANK_SIZE, LOROM_HEADER_OFFSET,
};
use crate::rom::Cartridge;
use crate::rom::header::mapping_mode::MappingMode;
use common::u16_split::*;
use std::io::Write;
use tempfile::tempdir;

#[cfg(not(tarpaulin_include))]
pub fn create_valid_header(map: MappingMode) -> Vec<u8> {
    let mut header = vec![0u8; HEADER_SIZE];

    let title: &[u8; 21] = b"TEST LOROM           "; // 21 bytes
    debug_assert!(title.len() == 21);
    header[0..21].copy_from_slice(title);

    // ROM Speed + Map Mode
    header[21] = match map {
        MappingMode::LoRom => 0x20, // FastROM + LoROM
        MappingMode::HiRom => 0x21, // FastROM + HiROM
    };
    header[22] = 0x00; // Cartridge type (no co-processor)
    header[23] = 0x08; // ROM size exponent (8 => 256 KB)
    header[24] = 0x00; // SRAM size (none)
    header[25] = 0x01; // Country (01 = USA NTSC)
    header[26] = 0x33; // Licensee code (Nintendo standard)
    header[27] = 0x00; // Version (0 => original release)

    // Checksum (dummy values for now)
    let checksum: u16 = 0xFFFF;
    let complement: u16 = !checksum;

    header[28] = *complement.lo();
    header[29] = *complement.hi();
    header[30] = *checksum.lo();
    header[31] = *checksum.hi();

    // Interruption Vectors (empty)
    header[32..HEADER_SIZE - 1].fill(0);

    header
}

#[cfg(not(tarpaulin_include))]
pub fn create_valid_lorom(size: usize) -> Vec<u8> {
    assert!(size >= LOROM_BANK_SIZE, "ROM must be at least 32KiB");
    let mut rom = vec![0; size];

    let header = create_valid_header(MappingMode::LoRom);
    rom[LOROM_HEADER_OFFSET..LOROM_HEADER_OFFSET + header.len()].copy_from_slice(&header);

    rom
}

#[cfg(not(tarpaulin_include))]
pub fn create_valid_hirom(size: usize) -> Vec<u8> {
    assert!(size >= HIROM_BANK_SIZE, "ROM must be at least 64KiB");
    let mut rom = vec![0; size];

    let header = create_valid_header(MappingMode::HiRom);
    rom[HIROM_HEADER_OFFSET..HIROM_HEADER_OFFSET + header.len()].copy_from_slice(&header);

    rom
}

#[cfg(not(tarpaulin_include))]
pub fn create_temp_rom(data: &[u8]) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let rom_path = dir.path().join("test_rom.sfc");
    let mut f = std::fs::File::create(&rom_path).unwrap();
    f.write_all(data).unwrap();

    (rom_path, dir)
}

pub fn with_sram(mut data: Vec<u8>, header_offset: usize, size_exp: u8) -> Vec<u8> {
    data[header_offset + HEADER_ROM_HARDWARE_OFFSET] = 0x02; // ROM + RAM + battery
    data[header_offset + HEADER_RAM_SIZE_OFFSET] = size_exp;
    data
}

pub fn lorom_with_sram(size_exp: u8) -> Cartridge {
    let data = with_sram(create_valid_lorom(0x10000), LOROM_HEADER_OFFSET, size_exp);
    let (path, _dir) = create_temp_rom(&data);
    Cartridge::load_from_file(path).unwrap()
}

pub fn hirom_with_sram(size_exp: u8) -> Cartridge {
    let data = with_sram(create_valid_hirom(0x10000), HIROM_HEADER_OFFSET, size_exp);
    let (path, _dir) = create_temp_rom(&data);
    Cartridge::load_from_file(path).unwrap()
}

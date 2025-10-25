//! Module which contains utility functions for
//! writing unit tests needing ROM objects

use crate::constants::{
    HEADER_SIZE, HIROM_BANK_SIZE, HIROM_HEADER_OFFSET, LOROM_BANK_SIZE, LOROM_HEADER_OFFSET,
};
use crate::rom::mapping_mode::MappingMode;
use std::io::Write;
use tempfile::tempdir;

pub(crate) fn create_valid_header(map: MappingMode) -> Vec<u8> {
    let mut header = vec![0u8; HEADER_SIZE];

    let title = b"TEST LOROM           "; // 21 bytes
    debug_assert!(title.len() == 21);
    header[0..21].copy_from_slice(title);

    // ROM Speed + Map Mode
    header[21] = match map {
        MappingMode::LoRom => 0x20, // FastROM + LoROM
        MappingMode::HiRom => 0x30, // FastROM + HiROM
        _ => 0x00,
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

    header[28] = (complement & 0xFF) as u8;
    header[29] = (complement >> 8) as u8;
    header[30] = (checksum & 0xFF) as u8;
    header[31] = (checksum >> 8) as u8;

    // Interruption Vectors (empty)
    header[32..HEADER_SIZE - 1].fill(0);

    header
}

pub(crate) fn create_valid_lorom(size: usize) -> Vec<u8> {
    assert!(size >= LOROM_BANK_SIZE, "ROM must be at least 32KiB");
    let mut rom = vec![0u8; size];

    let header = create_valid_header(MappingMode::LoRom);
    rom[LOROM_HEADER_OFFSET..LOROM_HEADER_OFFSET + header.len()].copy_from_slice(&header);

    rom
}

pub(crate) fn create_valid_hirom(size: usize) -> Vec<u8> {
    assert!(size >= HIROM_BANK_SIZE, "ROM must be at least 64KiB");
    let mut rom = vec![0u8; size];

    let header = create_valid_header(MappingMode::HiRom);
    rom[HIROM_HEADER_OFFSET..HIROM_HEADER_OFFSET + header.len()].copy_from_slice(&header);

    rom
}

pub(crate) fn create_temp_rom(data: &[u8]) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let rom_path = dir.path().join("test_rom.sfc");
    let mut f = std::fs::File::create(&rom_path).unwrap();
    f.write_all(data).unwrap();

    (rom_path, dir)
}

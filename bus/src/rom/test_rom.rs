//! Module which contains utility functions for
//! writing unit tests needing ROM objects

use std::io::Write;
use tempfile::tempdir;

pub(crate) fn create_valid_lorom(size: usize) -> Vec<u8> {
    assert!(size >= 0x8000, "ROM must be at least 32KiB");
    let mut rom = vec![0u8; size];

    let base = 0x7FC0;
    let title = b"TEST LOROM          "; // 21 bytes
    rom[base..base + title.len()].copy_from_slice(title);

    // LoROM mode
    rom[0x7FD5] = 0x20;
    rom[0x7FD6] = 0x00; // Cartridge type
    let rom_size_code = (size as f64 / 2048.0).log2().round() as u8;
    rom[0x7FD7] = rom_size_code;
    rom[0x7FD8] = 0x00; // SRAM size
    rom[0x7FD9] = 0x01; // Country
    rom[0x7FDA] = 0x33; // License
    rom[0x7FDB] = 0x00; // Version

    // Checksum
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

pub(crate) fn create_valid_hirom(size: usize) -> Vec<u8> {
    assert!(size >= 0x10000, "ROM must be at least 64KiB");
    let mut rom = vec![0u8; size];

    let base = 0xFFC0;
    let title = b"TEST HIROM          "; // 21 bytes
    rom[base..base + title.len()].copy_from_slice(title);

    // HiROM mode
    rom[0xFFD5] = 0x21; // HiROM mapping
    rom[0xFFD6] = 0x00;
    let rom_size_code = (size as f64 / 2048.0).log2().round() as u8;
    rom[0xFFD7] = rom_size_code;
    rom[0xFFD8] = 0x00;
    rom[0xFFD9] = 0x01;
    rom[0xFFDA] = 0x33;
    rom[0xFFDB] = 0x00;

    let sum: u16 = rom
        .iter()
        .enumerate()
        .filter(|(i, _)| !matches!(*i, 0xFFDC | 0xFFDD | 0xFFDE | 0xFFDF))
        .map(|(_, &b)| b as u32)
        .sum::<u32>() as u16;
    let checksum = sum;
    let complement = !checksum;

    rom[0xFFDC] = (complement & 0xFF) as u8;
    rom[0xFFDD] = (complement >> 8) as u8;
    rom[0xFFDE] = (checksum & 0xFF) as u8;
    rom[0xFFDF] = (checksum >> 8) as u8;

    rom
}

/// Helper: crée un fichier ROM temporaire avec des données arbitraires
pub(crate) fn create_temp_rom(data: &[u8]) -> std::path::PathBuf {
    let dir = tempdir().unwrap();
    let rom_path = dir.path().join("test_rom.sfc");
    let mut f = std::fs::File::create(&rom_path).unwrap();
    f.write_all(data).unwrap();
    std::mem::forget(dir); // éviter suppression du répertoire
    rom_path
}

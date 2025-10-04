use crate::constants::{BANK_SIZE, COPIER_HEADER_SIZE, LOROM_BANK_SIZE};
use crate::memory_region::MemoryRegion;
use crate::rom::error::RomError;
use crate::rom::mapping_mode::MappingMode;
use common::snes_address::SnesAddress;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// The game cartridge ROM contains the program code and data of the SNES game.
/// Its size varies by game (commonly 4 MiB or less, but can be larger with special chips).
///
/// The ROM can be mapped in two main modes:
/// - LoROM: 32 KiB of ROM is mapped into the upper half ($8000–$FFFF) of each bank.
///   Accessible in banks 0x00–0x7D and 0x80–0xFF. Each bank contributes 32 KiB to the ROM.
/// - HiROM: 64 KiB of ROM is mapped into the full range ($0000–$FFFF) of each bank.
///   Accessible in banks 0x00–0x3F and 0x80–0xBF. Each bank contributes 64 KiB to the ROM.
///
/// Some cartridges may contain a 512-byte copier header at the start of the file,
/// which is removed on load.
/// ROM data is read-only and any write attempts are ignored.
#[derive(PartialEq, Debug)]
pub struct Rom {
    pub data: Vec<u8>,
    pub map: MappingMode,
}

impl Rom {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, RomError> {
        let mut file = File::open(path).map_err(RomError::IoError)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(RomError::IoError)?;

        if buffer.len() < LOROM_BANK_SIZE {
            return Err(RomError::FileTooSmall);
        }

        // Check for 512-byte header
        let rom_data = if buffer.len() % LOROM_BANK_SIZE == COPIER_HEADER_SIZE {
            buffer[COPIER_HEADER_SIZE..].to_vec() // Remove useless "Copier" 512-byte header
        } else {
            buffer.to_vec()
        };

        // Check map mode
        let map_mode = MappingMode::detect_rom_mapping(&rom_data);

        Ok(Rom {
            data: rom_data,
            map: map_mode,
        })
    }

    #[allow(dead_code)]
    pub fn new(data: Vec<u8>, map: MappingMode) -> Self {
        Self { data, map }
    }

    fn panic_invalid_addr(addr: SnesAddress) -> ! {
        panic!(
            "Incorrect access to the ROM at address: {:02X}{:04X}",
            addr.bank, addr.addr
        );
    }

    /// Converts a `SnesAddress` into an internal LoROM ROM offset.
    ///
    /// Maps the SNES ROM address space for LoROM cartridges:
    /// - Banks $00–$7D and $80–$FF, addresses $8000–$FFFF → regular ROM area.
    /// - Banks $40–$7D and $C0–$FF, addresses $0000–$7FFF → mirrors of upper half.
    ///
    /// Each bank contributes 32 KiB to the ROM.
    ///
    /// # Panics
    /// Panics if the given address does not correspond to a valid LoROM location.
    fn get_lorom_offset(&self, addr: SnesAddress) -> usize {
        match addr.bank {
            // Banks 00-7D and 80-FF
            0x00..=0x7D | 0x80..=0xFF => {
                if addr.addr >= 0x8000 {
                    // Regular LoROM region
                    let bank_index = if addr.bank <= 0x7D {
                        addr.bank as usize
                    } else {
                        addr.bank as usize - 0x80
                    };
                    let bank_offset = (addr.addr - 0x8000) as usize;
                    return bank_index * 0x8000 + bank_offset;
                } else if (0x40..=0x7D).contains(&addr.bank) || (0xC0..=0xFF).contains(&addr.bank) {
                    // Mirror of upper half: 0000-7FFF → 8000-FFFF
                    let bank_index = if addr.bank <= 0x7D {
                        addr.bank as usize - 0x40
                    } else {
                        addr.bank as usize - 0xC0
                    };
                    let bank_offset = addr.addr as usize;
                    return bank_index * 0x8000 + bank_offset;
                } else {
                    Self::panic_invalid_addr(addr);
                }
            }
            _ => {
                Self::panic_invalid_addr(addr);
            }
        }
    }

    /// Converts a `SnesAddress` into an internal HiROM ROM offset.
    ///
    /// Maps the SNES ROM address space for HiROM cartridges:
    /// - Banks $40–$7D and $C0–$FF, addresses $0000–$FFFF → regular ROM area (full 64 KiB per bank).
    /// - Banks $00–$3F and $80–$BF, addresses $8000–$FFFF → mirrors of the full bank.
    ///
    /// Each bank contributes 64 KiB to the ROM.
    ///
    /// # Panics
    /// Panics if the given address does not correspond to a valid HiROM location.
    fn get_hirom_offset(&self, addr: SnesAddress) -> usize {
        match addr.bank {
            // Regular HiROM banks
            0x40..=0x7D => {
                let bank_index = addr.bank as usize - 0x40;
                return bank_index * BANK_SIZE + addr.addr as usize;
            }
            0xC0..=0xFF => {
                let bank_index = addr.bank as usize - 0xC0;
                return bank_index * BANK_SIZE + addr.addr as usize;
            }

            // Mirror Banks
            0x00..=0x3F => {
                if addr.addr >= 0x8000 {
                    let bank_index = addr.bank as usize - 0x00;
                    let bank_offset = (addr.addr - 0x8000) as usize; // TODO : Maybe the "- 0x8000" is not necessary
                    return bank_index * BANK_SIZE + bank_offset;
                } else {
                    Self::panic_invalid_addr(addr);
                }
            }
            0x80..=0xBF => {
                if addr.addr >= 0x8000 {
                    let bank_index = addr.bank as usize - 0x80;
                    let bank_offset = (addr.addr - 0x8000) as usize; // TODO : Maybe the "- 0x8000" is not necessary
                    return bank_index * BANK_SIZE + bank_offset;
                } else {
                    Self::panic_invalid_addr(addr);
                }
            }

            _ => {
                Self::panic_invalid_addr(addr);
            }
        }
    }

    // TODO : take in account ROM mirrors :
    // https://problemkaputt.de/fullsnes.htm#snescarthirommappingromdividedinto64kbanksaround500games
    // fn get_hirom_offset(&self, addr: SnesAddress) -> usize {
    //     match addr.bank {
    //         0x00..=0x3F | 0x80..=0xBF => {
    //             let bank_index = (addr.bank & 0x3F) as usize;
    //             return bank_index * 0x10000 + addr.addr as usize;
    //         }
    //         _ => {
    //             Self::panic_invalid_addr(addr);
    //         }
    //     }
    // }

    // TODO : take in account ROM mirrors :
    // https://problemkaputt.de/fullsnes.htm#snescartlorommappingromdividedinto32kbanksaround1500games
    // fn get_lorom_offset(&self, addr: SnesAddress) -> usize {
    //     match addr.bank {
    //         0x00..=0x7D | 0x80..=0xFF => {
    //             if addr.addr >= 0x8000 {
    //                 let bank_index = (addr.bank & 0x7F) as usize;
    //                 let bank_offset = (addr.addr - 0x8000) as usize;
    //                 return bank_index * LOROM_BANK_SIZE + bank_offset;
    //             } else {
    //                 Self::panic_invalid_addr(addr);
    //             }
    //         }
    //         _ => {
    //             Self::panic_invalid_addr(addr);
    //         }
    //     }
    // }

    /// Converts a `SnesAddress` into an internal ROM offset.
    ///
    /// Uses the ROM’s mapping mode (`MappingMode::LoRom` or `MappingMode::HiRom`)
    /// to compute the correct byte position in the loaded ROM data.
    ///
    /// # Panics
    /// Panics if the address is invalid for the detected mapping mode,
    /// or if the mapping mode is [`MappingMode::Unknown`].
    fn to_offset(&self, addr: SnesAddress) -> usize {
        match self.map {
            MappingMode::HiRom => self.get_hirom_offset(addr),
            MappingMode::LoRom => self.get_lorom_offset(addr),
            MappingMode::Unknown => {
                panic!("ROM mapping mode is Unknown, cannot compute offset");
            }
        }
    }
}

impl MemoryRegion for Rom {
    /// Reads a byte from the ROM at the given `SnesAddress`.
    ///
    /// The address is translated to an internal ROM offset using `to_offset`.
    ///
    /// # Panics
    /// Panics if the mapping mode is `MappingMode::Unknown` or index out of bounds.
    fn read(&self, addr: SnesAddress) -> u8 {
        let offset = self.to_offset(addr);

        return self.data.get(offset).copied().expect(&format!(
            "ERROR: Couldn't extract value from ROM at address: {:02X}{:04X}",
            addr.bank, addr.addr
        ));
    }

    /// Ignores writes to the ROM.
    ///
    /// ROM is read-only; this function performs no action.
    fn write(&mut self, _addr: SnesAddress, _value: u8) {
        // ROM is read-only, ignore writes
        // TODO : Add a warning ?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{COPIER_HEADER_SIZE, LOROM_BANK_SIZE};
    use crate::rom::mapping_mode::MappingMode;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_valid_lorom(size: usize) -> Vec<u8> {
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

    fn create_valid_hirom(size: usize) -> Vec<u8> {
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
    fn create_temp_rom(data: &[u8]) -> std::path::PathBuf {
        let dir = tempdir().unwrap();
        let rom_path = dir.path().join("test_rom.sfc");
        let mut f = std::fs::File::create(&rom_path).unwrap();
        f.write_all(data).unwrap();
        std::mem::forget(dir); // éviter suppression du répertoire
        rom_path
    }

    #[test]
    fn test_detect_lorom() {
        let data = create_valid_lorom(0x10000);
        let path = create_temp_rom(&data);

        let rom = Rom::load_from_file(path).unwrap();
        assert_eq!(rom.map, MappingMode::LoRom);
    }

    #[test]
    fn test_detect_hirom() {
        let data = create_valid_hirom(0x10000);
        let path = create_temp_rom(&data);

        let rom = Rom::load_from_file(path).unwrap();
        assert_eq!(rom.map, MappingMode::HiRom);
    }

    #[test]
    fn test_load_rom_success() {
        let data = create_valid_lorom(0x10000);
        let path = create_temp_rom(&data);

        let rom = Rom::load_from_file(&path).unwrap();
        assert_eq!(rom.data.len(), data.len());
        // assert_ne!(rom.map, MappingMode::Unknown);
    }

    #[test]
    fn test_load_rom_with_copier_header() {
        let mut data = create_valid_lorom(LOROM_BANK_SIZE + COPIER_HEADER_SIZE);
        // Simule header inutile devant
        for b in &mut data[..COPIER_HEADER_SIZE] {
            *b = 0xFF;
        }
        let path = create_temp_rom(&data);

        let rom = Rom::load_from_file(&path).unwrap();
        // Vérifie qu'on a bien retiré le header
        assert_eq!(rom.data.len(), LOROM_BANK_SIZE);
        assert_eq!(rom.data[0], 0);
    }

    #[test]
    fn test_load_rom_too_small() {
        let data = vec![0x00; LOROM_BANK_SIZE - 1];
        let path = create_temp_rom(&data);
        let result = Rom::load_from_file(&path);
        assert!(matches!(result, Err(RomError::FileTooSmall)));
    }

    #[test]
    #[should_panic(expected = "ROM mapping mode is Unknown")]
    fn test_to_offset_unknown_panics() {
        let rom = Rom::new(vec![0; LOROM_BANK_SIZE], MappingMode::Unknown);
        rom.to_offset(SnesAddress {
            bank: 0x00,
            addr: 0x8000,
        });
    }

    #[test]
    fn test_write_is_ignored() {
        let data = create_valid_lorom(0x10000);
        let path = create_temp_rom(&data);
        let mut rom = Rom::load_from_file(&path).unwrap();

        let addr = SnesAddress {
            bank: 0x00,
            addr: 0x8000,
        };
        rom.write(addr, 0x99);
        assert_eq!(rom.read(addr), 0);
    }

    // #[test]
    // fn test_hirom_offset_and_read() {
    //     // let data = (0..0x20000).map(|x| (x & 0xFF) as u8).collect::<Vec<_>>();
    //     // let rom = Rom::new(data.clone(), MappingMode::HiRom);
    //     let data = create_valid_hirom(0x20000);
    //     let path = create_temp_rom(&data);
    //     let rom = Rom::load_from_file(&path).unwrap();

    //     let addr = SnesAddress {
    //         bank: 0xC0,
    //         addr: 0x1234,
    //     };
    //     let offset = rom.to_offset(addr);
    //     assert_eq!(rom.read(addr), data[offset]);
    // }

    // #[test]
    // fn test_lorom_offset_and_read() {
    //     // 2 banks de 32 KiB
    //     let data = (0..0x10000).map(|x| (x & 0xFF) as u8).collect::<Vec<_>>();
    //     let rom = Rom::new(data.clone(), MappingMode::LoRom);

    //     let addr = SnesAddress {
    //         bank: 0x01,
    //         addr: 0x8000,
    //     }; // début 2ème banque
    //     let offset = rom.to_offset(addr);
    //     assert_eq!(offset, LOROM_BANK_SIZE); // doit pointer au début de la 2ème banque
    //     assert_eq!(rom.read(addr), data[offset]);
    // }
}

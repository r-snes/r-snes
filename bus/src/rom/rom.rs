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
        let map_mode =
            MappingMode::detect_rom_mapping(&rom_data).ok_or(RomError::IncorrectMapping)?;

        Ok(Rom {
            data: rom_data,
            map: map_mode,
        })
    }

    fn panic_invalid_addr(addr: SnesAddress) -> ! {
        panic!(
            "Incorrect access to the ROM at address: {:06X}",
            usize::from(addr)
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
    pub fn get_lorom_offset(addr: SnesAddress) -> usize {
        match addr.bank {
            0x00..=0x3F if addr.addr >= 0x8000 => {
                let bank_offset = (addr.addr - 0x8000) as usize;
                return addr.bank as usize * 0x8000 + bank_offset;
            }
            0x40..=0x7D => {
                if addr.addr < 0x8000 {
                    // ROM Mirror
                    let bank_offset = addr.addr as usize;
                    return addr.bank as usize * 0x8000 + bank_offset;
                } else {
                    // Superior or equal to 0x8000
                    let bank_offset = (addr.addr - 0x8000) as usize;
                    return addr.bank as usize * 0x8000 + bank_offset;
                }
            }
            0x80..=0xBF if addr.addr >= 0x8000 => {
                let bank_offset = (addr.addr - 0x8000) as usize;
                return (addr.bank as usize - 0x80) * 0x8000 + bank_offset;
            }
            0xC0..=0xFF => {
                if addr.addr < 0x8000 {
                    // ROM Mirror
                    let bank_offset = addr.addr as usize;
                    return (addr.bank as usize - 0x80) * 0x8000 + bank_offset;
                } else {
                    // Superior or equal to 0x8000
                    let bank_offset = (addr.addr - 0x8000) as usize;
                    return (addr.bank as usize - 0x80) * 0x8000 + bank_offset;
                }
            }
            _ => Self::panic_invalid_addr(addr),
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
    pub fn get_hirom_offset(addr: SnesAddress) -> usize {
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
            0x00..=0x3F if addr.addr >= 0x8000 => {
                let bank_index = addr.bank as usize - 0x00;
                let bank_offset = addr.addr as usize;
                return bank_index * BANK_SIZE + bank_offset;
            }
            0x80..=0xBF if addr.addr >= 0x8000 => {
                let bank_index = addr.bank as usize - 0x80;
                let bank_offset = addr.addr as usize;
                return bank_index * BANK_SIZE + bank_offset;
            }
            _ => Self::panic_invalid_addr(addr),
        }
    }

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
            MappingMode::HiRom => Self::get_hirom_offset(addr),
            MappingMode::LoRom => Self::get_lorom_offset(addr),
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

        return *self.data.get(offset).expect(&format!(
            "ERROR: Couldn't extract value from ROM at address: {:06X}",
            usize::from(addr)
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
    use crate::constants::{COPIER_HEADER_SIZE, HIROM_BANK_SIZE, LOROM_BANK_SIZE};
    use crate::rom::mapping_mode::MappingMode;
    use crate::rom::test_rom::*;
    use common::snes_address::snes_addr;

    #[test]
    fn test_detect_lorom() {
        let data = create_valid_lorom(0x10000);
        let (path, _dir) = create_temp_rom(&data);

        let rom = Rom::load_from_file(path).unwrap();
        assert_eq!(rom.map, MappingMode::LoRom);
        assert_eq!(rom.read(snes_addr!(0x00:0x8000)), 0);
    }

    #[test]
    fn test_detect_hirom() {
        let data = create_valid_hirom(0x10000);
        let (path, _dir) = create_temp_rom(&data);

        let rom = Rom::load_from_file(path).unwrap();
        assert_eq!(rom.map, MappingMode::HiRom);
        assert_eq!(rom.read(snes_addr!(0x00:0x8000)), 0);
    }

    #[test]
    fn test_load_rom_success() {
        let data = create_valid_lorom(0x10000);
        let (path, _dir) = create_temp_rom(&data);

        let rom = Rom::load_from_file(&path).unwrap();
        assert_eq!(rom.data.len(), data.len());
    }

    #[test]
    fn test_load_rom_with_copier_header() {
        let data = create_valid_lorom(HIROM_BANK_SIZE);
        let mut copier_header_data: Vec<u8> = vec![0xFF; COPIER_HEADER_SIZE];
        copier_header_data.extend_from_slice(&data);

        let (path, _dir) = create_temp_rom(&copier_header_data);
        let rom = Rom::load_from_file(&path).unwrap();

        // Check copier header removed
        assert_eq!(rom.data.len(), HIROM_BANK_SIZE);
        assert_eq!(rom.data[0], 0);
    }

    #[test]
    fn test_load_rom_too_small() {
        let data = vec![0x00; LOROM_BANK_SIZE - 1];
        let (path, _dir) = create_temp_rom(&data);
        let result = Rom::load_from_file(&path);
        assert!(matches!(result, Err(RomError::FileTooSmall)));
    }

    #[test]
    fn test_write_is_ignored() {
        let data = create_valid_lorom(0x10000);
        let (path, _dir) = create_temp_rom(&data);
        let mut rom = Rom::load_from_file(&path).unwrap();

        let addr = snes_addr!(0x00:0x8000);
        rom.write(addr, 0x99);
        assert_eq!(rom.read(addr), 0);
    }

    #[test]
    fn test_lorom_offset_first_quarter() {
        let mut addr = snes_addr!(0x00:0x8000);
        assert_eq!(Rom::get_lorom_offset(addr), 0);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 - 1);

        addr.bank = 0x01;
        addr.addr = 0x8000;
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_lorom_offset(addr), 0x10000 - 1);

        addr.bank = 0x3F;
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x3F + 1) - 1);
    }

    #[test]
    fn test_lorom_offset_second_quarter() {
        let mut addr = snes_addr!(0x40:0x8000);
        let mut mirror_addr = snes_addr!(0x40:0x0);

        assert_eq!(
            Rom::get_lorom_offset(addr),
            Rom::get_lorom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x40));

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0x7FFF;
        assert_eq!(
            Rom::get_lorom_offset(addr),
            Rom::get_lorom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x40 + 1) - 1);

        addr.addr = 0x8000;
        mirror_addr.addr = 0x0000;
        addr.bank = 0x7D;
        mirror_addr.bank = 0x7D;
        assert_eq!(
            Rom::get_lorom_offset(addr),
            Rom::get_lorom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x7D));

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0x7FFF;
        assert_eq!(
            Rom::get_lorom_offset(addr),
            Rom::get_lorom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x7D + 1) - 1);
    }

    #[test]
    fn test_lorom_offset_third_quarter() {
        let mut addr = snes_addr!(0x80:0x8000);
        assert_eq!(Rom::get_lorom_offset(addr), 0);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 - 1);

        addr.bank = 0x81;
        addr.addr = 0x8000;
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_lorom_offset(addr), 0x10000 - 1);

        addr.bank = 0xBF;
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x3F + 1) - 1);
    }

    #[test]
    fn test_lorom_offset_fourth_quarter() {
        let mut addr = snes_addr!(0xC0:0x8000);
        let mut mirror_addr = snes_addr!(0xC0:0x0);

        assert_eq!(
            Rom::get_lorom_offset(addr),
            Rom::get_lorom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x40));

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0x7FFF;
        assert_eq!(
            Rom::get_lorom_offset(addr),
            Rom::get_lorom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x40 + 1) - 1);

        addr.addr = 0x8000;
        mirror_addr.addr = 0x0000;
        addr.bank = 0xFF;
        mirror_addr.bank = 0xFF;
        assert_eq!(
            Rom::get_lorom_offset(addr),
            Rom::get_lorom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x7D + 2));

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0x7FFF;
        assert_eq!(
            Rom::get_lorom_offset(addr),
            Rom::get_lorom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_lorom_offset(addr), 0x8000 * (0x7D + 3) - 1);
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the ROM at address: 004000")]
    fn test_lorom_incorrect_address() {
        let addr = snes_addr!(0x00:0x4000);
        assert_eq!(Rom::get_lorom_offset(addr), 0);
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the ROM at address: 804000")]
    fn test_lorom_incorrect_address2() {
        let addr = snes_addr!(0x80:0x4000);
        assert_eq!(Rom::get_lorom_offset(addr), 0);
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the ROM at address: 7E4000")]
    fn test_lorom_incorrect_address3() {
        let addr = snes_addr!(0x7E:0x4000);
        assert_eq!(Rom::get_lorom_offset(addr), 0);
    }

    #[test]
    fn test_hirom_offset_first_quarter() {
        let mut addr = snes_addr!(0x40:0x8000);
        let mut mirror_addr = snes_addr!(0x00:0x8000);

        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0x8000);

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0xFFFF);

        addr.addr = 0x8000;
        mirror_addr.addr = 0x8000;
        addr.bank = 0x7D;
        mirror_addr.bank = 0x3D;
        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0x10000 * (0x3D) + 0x8000);

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        addr.bank = 0x7D;
        mirror_addr.bank = 0x3D;
        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0x10000 * (0x3D) + 0xFFFF);

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        addr.bank = 0xFF;
        mirror_addr.bank = 0x3F;
        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0x10000 * (0x3F) + 0xFFFF);
    }

    #[test]
    fn test_hirom_offset_second_quarter() {
        let mut addr = snes_addr!(0x40:0x0000);
        assert_eq!(Rom::get_hirom_offset(addr), 0);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_hirom_offset(addr), 0xFFFF);

        addr.bank = 0x41;
        addr.addr = 0x0000;
        assert_eq!(Rom::get_hirom_offset(addr), 0x10000);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_hirom_offset(addr), 0x10000 * 2 - 1);

        addr.bank = 0x7D;
        assert_eq!(Rom::get_hirom_offset(addr), 0x10000 * (0x3D + 1) - 1);
    }

    #[test]
    fn test_hirom_offset_third_quarter() {
        let mut addr = snes_addr!(0xC0:0x8000);
        let mut mirror_addr = snes_addr!(0x80:0x8000);

        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0x8000);

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0xFFFF);

        addr.addr = 0x8000;
        mirror_addr.addr = 0x8000;
        addr.bank = 0xFF;
        mirror_addr.bank = 0xBF;
        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0x10000 * (0x3F) + 0x8000);

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        addr.bank = 0xFF;
        mirror_addr.bank = 0xBF;
        assert_eq!(
            Rom::get_hirom_offset(addr),
            Rom::get_hirom_offset(mirror_addr)
        );
        assert_eq!(Rom::get_hirom_offset(addr), 0x10000 * (0x3F) + 0xFFFF);
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the ROM at address: 004000")]
    fn test_hirom_incorrect_address() {
        let addr = snes_addr!(0x00:0x4000);
        assert_eq!(Rom::get_hirom_offset(addr), 0);
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the ROM at address: 804000")]
    fn test_hirom_incorrect_address2() {
        let addr = snes_addr!(0x80:0x4000);
        assert_eq!(Rom::get_hirom_offset(addr), 0);
    }

    #[test]
    #[should_panic(expected = "Incorrect access to the ROM at address: 7E4000")]
    fn test_hirom_incorrect_address3() {
        let addr = snes_addr!(0x7E:0x4000);
        assert_eq!(Rom::get_hirom_offset(addr), 0);
    }
}

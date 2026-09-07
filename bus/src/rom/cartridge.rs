use crate::constants::{BANK_SIZE, COPIER_HEADER_SIZE, LOROM_BANK_SIZE};
use crate::rom::error::RomError;
use crate::rom::header::RomHeader;
use crate::rom::header::mapping_mode::MappingMode;
use crate::rom::sram::Sram;
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
#[derive(PartialEq)]
pub struct Rom {
    pub data: Vec<u8>,
    pub map: MappingMode,
    pub header: RomHeader,
    pub sram: Sram,
}

/// Which chip on the cartridge board responds to a given address.
///
/// Returns the corresponding offset into the ROM or S-RAM, or `Unmapped` if no chip responds.
enum CartridgeTarget {
    /// Byte offset into the mask ROM.
    Rom(usize),
    /// Address on the S-RAM chip
    Sram(usize),
    Unmapped,
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
        let header = RomHeader::load_header(&rom_data, map_mode);

        // Detect if found mapping and header mapping are different
        if map_mode != header.mapping_mode {
            return Err(RomError::IncorrectMapping);
        }

        Ok(Rom {
            data: rom_data,
            map: map_mode,
            sram: Sram::new(&header),
            header,
        })
    }

    /// Determines which chip on the cartridge board responds to a `SnesAddress`.
    ///
    /// Returns the selected chip and the offset within it, or
    /// `CartridgeTarget::Unmapped` if no chip drives the data bus.
    fn decode(&self, addr: SnesAddress) -> CartridgeTarget {
        match self.map {
            MappingMode::LoRom => self.decode_lorom(addr),
            MappingMode::HiRom => Self::decode_hirom(addr),
        }
    }

    /// Determines which chip on a LoROM cartridge board responds to a `SnesAddress`.
    ///
    /// Returns the selected chip and the offset within it, or
    /// `CartridgeTarget::Unmapped` if no chip drives the data bus.
    fn decode_lorom(&self, addr: SnesAddress) -> CartridgeTarget {
        if self.sram.is_present() {
            if matches!(
                (addr.bank, addr.addr),
                (0x70..=0x7D | 0xF0..=0xFF, 0x0000..=0x7FFF)
            ) {
                return CartridgeTarget::Sram(addr.addr as usize);
            }
        }

        match Self::get_lorom_offset(addr) {
            Some(offset) => CartridgeTarget::Rom(offset),
            None => CartridgeTarget::Unmapped,
        }
    }

    /// Determines which chip on a HiROM cartridge board responds to a `SnesAddress`.
    ///
    /// Returns the selected chip and the offset within it, or
    /// `CartridgeTarget::Unmapped` if no chip drives the data bus.
    fn decode_hirom(addr: SnesAddress) -> CartridgeTarget {
        if let (0x20..=0x3F | 0xA0..=0xBF, 0x6000..=0x7FFF) = (addr.bank, addr.addr) {
            // AND with 0x3F to fold $A0-$BF back onto $20-$3F
            let bank = addr.bank as usize & 0x3F;

            return CartridgeTarget::Sram((bank << 13) | (addr.addr as usize & 0x1FFF));
        }

        match Self::get_hirom_offset(addr) {
            Some(offset) => CartridgeTarget::Rom(offset),
            None => CartridgeTarget::Unmapped,
        }
    }

    /// Converts a `SnesAddress` into an internal LoROM ROM offset.
    ///
    /// Maps the SNES ROM address space for LoROM cartridges:
    /// - The upper half of banks $80-$FF map the entire ROM one-to-one
    /// - The upper half of banks $00-$7D mirror what is in $80-$FD
    /// - Banks $40-$7D and $C0-FF ignore the highest bit of the address,
    ///   and mirror the same mapping as $80-$FF
    ///
    /// Each bank maps 32 distinct KiB of the ROM.
    ///
    /// Returns `None` if the address does not select the mask ROM.
    pub fn get_lorom_offset(addr: SnesAddress) -> Option<usize> {
        match (addr.bank, addr.addr) {
            | (0x00..=0x7D, 0x8000..=0xFFFF)
            | (0x80..=0xFF, 0x8000..=0xFFFF)
            | (0x40..=0x7D, _)
            | (0xC0..=0xFF, _) => {
                let bank = addr.bank & !0x80;
                let addr = addr.addr & !0x8000;

                Some(bank as usize * LOROM_BANK_SIZE + addr as usize)
            }
            _ => None,
        }
    }

    /// Converts a `SnesAddress` into an internal HiROM ROM offset.
    ///
    /// Maps the SNES ROM address space for HiROM cartridges:
    /// - Banks $C0-$FF ($0000-$FFFF, full bank) map the entire ROM one-to-one
    /// - Banks $40-$FD ($0000-$FFFF, full bank) mirror banks $C0-$FD
    /// - Banks $00-3F and $80-BF ($8000-$FFFF, only upper half) mirror the
    ///   upper halves of $C0-$FF
    ///
    /// Returns `None` if the address does not select the mask ROM.
    pub fn get_hirom_offset(addr: SnesAddress) -> Option<usize> {
        match (addr.bank, addr.addr) {
            | (0x00..=0x7D, 0x8000..=0xFFFF)
            | (0x80..=0xFF, 0x8000..=0xFFFF)
            | (0x40..=0x7D, _)
            | (0xC0..=0xFF, _) => {
                // AND with 0x3F so that we start over from 0 every 0x40 (64) banks
                let bank = addr.bank as usize & 0x3F;

                Some(bank * BANK_SIZE + addr.addr as usize)
            }
            _ => None,
        }
    }

    /// Converts a `SnesAddress` into an internal ROM offset.
    ///
    /// Uses the ROM’s mapping mode (`MappingMode::LoRom` or `MappingMode::HiRom`)
    /// to compute the correct byte position in the loaded ROM data.
    ///
    /// Returns `None` if the address is invalid for the detected mapping mode.
    pub fn to_offset(&self, addr: SnesAddress) -> Option<usize> {
        match self.map {
            MappingMode::HiRom => Self::get_hirom_offset(addr),
            MappingMode::LoRom => Self::get_lorom_offset(addr),
        }
    }
}

impl Rom {
    /// Reads a byte from the cartridge at the given `SnesAddress`.
    ///
    /// Returns `None` when no chip on the board responds, leaving the caller
    /// to return open bus.
    pub fn read(&self, addr: SnesAddress) -> Option<u8> {
        match self.decode(addr) {
            CartridgeTarget::Rom(offset) => self.data.get(offset).copied(),
            CartridgeTarget::Sram(linear) => self.sram.read(linear),
            CartridgeTarget::Unmapped => None,
        }
    }

    /// Writes a byte to the cartridge at the given `SnesAddress`. Writes to the ROM are ignored.
    pub fn write(&mut self, addr: SnesAddress, value: u8) {
        match self.decode(addr) {
            CartridgeTarget::Sram(linear) => self.sram.write(linear, value),
            CartridgeTarget::Rom(_) | CartridgeTarget::Unmapped => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{COPIER_HEADER_SIZE, HIROM_BANK_SIZE, LOROM_BANK_SIZE};
    use crate::rom::header::mapping_mode::MappingMode;
    use crate::rom::test_rom::*;
    use common::snes_address::snes_addr;

    #[test]
    fn test_detect_lorom() {
        let data = create_valid_lorom(0x10000);
        let (path, _dir) = create_temp_rom(&data);

        let rom = Rom::load_from_file(path).unwrap();
        assert_eq!(rom.map, MappingMode::LoRom);
        assert_eq!(rom.read(snes_addr!(0:0x8000)).unwrap(), 0);
    }

    #[test]
    fn test_detect_hirom() {
        let data = create_valid_hirom(0x10000);
        let (path, _dir) = create_temp_rom(&data);

        let rom = Rom::load_from_file(path).unwrap();
        assert_eq!(rom.map, MappingMode::HiRom);
        assert_eq!(rom.read(snes_addr!(0:0x8000)).unwrap(), 0);
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

        let addr = snes_addr!(0:0x8000);
        rom.write(addr, 0x99);
        assert_eq!(rom.read(addr).unwrap(), 0);
    }

    #[test]
    fn test_lorom_offset_first_quarter() {
        let mut addr = snes_addr!(0:0x8000);
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x8000 - 1);

        addr.bank = 0x01;
        addr.addr = 0x8000;
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x8000);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x10000 - 1);

        addr.bank = 0x3F;
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            0x8000 * (0x3F + 1) - 1
        );
    }

    #[test]
    fn test_lorom_offset_second_quarter() {
        let mut addr = snes_addr!(0x40:0x8000);
        let mut mirror_addr = snes_addr!(0x40:0x0);

        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            Rom::get_lorom_offset(mirror_addr).unwrap()
        );
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x8000 * (0x40));

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0x7FFF;
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            Rom::get_lorom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            0x8000 * (0x40 + 1) - 1
        );

        addr.addr = 0x8000;
        mirror_addr.addr = 0x0000;
        addr.bank = 0x7D;
        mirror_addr.bank = 0x7D;
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            Rom::get_lorom_offset(mirror_addr).unwrap()
        );
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x8000 * (0x7D));

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0x7FFF;
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            Rom::get_lorom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            0x8000 * (0x7D + 1) - 1
        );
    }

    #[test]
    fn test_lorom_offset_third_quarter() {
        let mut addr = snes_addr!(0x80:0x8000);
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x8000 - 1);

        addr.bank = 0x81;
        addr.addr = 0x8000;
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x8000);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x10000 - 1);

        addr.bank = 0xBF;
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            0x8000 * (0x3F + 1) - 1
        );
    }

    #[test]
    fn test_lorom_offset_fourth_quarter() {
        let mut addr = snes_addr!(0xC0:0x8000);
        let mut mirror_addr = snes_addr!(0xC0:0x0);

        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            Rom::get_lorom_offset(mirror_addr).unwrap()
        );
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x8000 * (0x40));

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0x7FFF;
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            Rom::get_lorom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            0x8000 * (0x40 + 1) - 1
        );

        addr.addr = 0x8000;
        mirror_addr.addr = 0x0000;
        addr.bank = 0xFF;
        mirror_addr.bank = 0xFF;
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            Rom::get_lorom_offset(mirror_addr).unwrap()
        );
        assert_eq!(Rom::get_lorom_offset(addr).unwrap(), 0x8000 * (0x7D + 2));

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0x7FFF;
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            Rom::get_lorom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_lorom_offset(addr).unwrap(),
            0x8000 * (0x7D + 3) - 1
        );
    }

    #[test]
    fn test_lorom_incorrect_address() {
        let addr = snes_addr!(0:0x4000);
        assert_eq!(Rom::get_lorom_offset(addr), None);
    }

    #[test]
    fn test_lorom_incorrect_address2() {
        let addr = snes_addr!(0x80:0x4000);
        assert_eq!(Rom::get_lorom_offset(addr), None);
    }

    #[test]
    fn test_lorom_incorrect_address3() {
        let addr = snes_addr!(0x7E:0x4000);
        assert_eq!(Rom::get_lorom_offset(addr), None);
    }

    #[test]
    fn test_hirom_offset_first_quarter() {
        let mut addr = snes_addr!(0x40:0x8000);
        let mut mirror_addr = snes_addr!(0:0x8000);

        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(Rom::get_hirom_offset(addr).unwrap(), 0x8000);

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(Rom::get_hirom_offset(addr).unwrap(), 0xFFFF);

        addr.addr = 0x8000;
        mirror_addr.addr = 0x8000;
        addr.bank = 0x7D;
        mirror_addr.bank = 0x3D;
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            0x10000 * (0x3D) + 0x8000
        );

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        addr.bank = 0x7D;
        mirror_addr.bank = 0x3D;
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            0x10000 * (0x3D) + 0xFFFF
        );

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        addr.bank = 0xFF;
        mirror_addr.bank = 0x3F;
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            0x10000 * (0x3F) + 0xFFFF
        );
    }

    #[test]
    fn test_hirom_offset_second_quarter() {
        let mut addr = snes_addr!(0x40:0x0000);
        assert_eq!(Rom::get_hirom_offset(addr).unwrap(), 0);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_hirom_offset(addr).unwrap(), 0xFFFF);

        addr.bank = 0x41;
        addr.addr = 0x0000;
        assert_eq!(Rom::get_hirom_offset(addr).unwrap(), 0x10000);

        addr.addr = 0xFFFF;
        assert_eq!(Rom::get_hirom_offset(addr).unwrap(), 0x10000 * 2 - 1);

        addr.bank = 0x7D;
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            0x10000 * (0x3D + 1) - 1
        );
    }

    #[test]
    fn test_hirom_offset_third_quarter() {
        let mut addr = snes_addr!(0xC0:0x8000);
        let mut mirror_addr = snes_addr!(0x80:0x8000);

        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(Rom::get_hirom_offset(addr).unwrap(), 0x8000);

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(Rom::get_hirom_offset(addr).unwrap(), 0xFFFF);

        addr.addr = 0x8000;
        mirror_addr.addr = 0x8000;
        addr.bank = 0xFF;
        mirror_addr.bank = 0xBF;
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            0x10000 * (0x3F) + 0x8000
        );

        addr.addr = 0xFFFF;
        mirror_addr.addr = 0xFFFF;
        addr.bank = 0xFF;
        mirror_addr.bank = 0xBF;
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            Rom::get_hirom_offset(mirror_addr).unwrap()
        );
        assert_eq!(
            Rom::get_hirom_offset(addr).unwrap(),
            0x10000 * (0x3F) + 0xFFFF
        );
    }

    #[test]
    fn test_hirom_incorrect_address() {
        let addr = snes_addr!(0:0x4000);
        assert_eq!(Rom::get_hirom_offset(addr), None);
    }

    #[test]
    fn test_hirom_incorrect_address2() {
        let addr = snes_addr!(0x80:0x4000);
        assert_eq!(Rom::get_hirom_offset(addr), None);
    }

    #[test]
    fn test_hirom_incorrect_address3() {
        let addr = snes_addr!(0x7E:0x4000);
        assert_eq!(Rom::get_hirom_offset(addr), None);
    }
}

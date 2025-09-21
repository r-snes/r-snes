use crate::constants::{COPIER_HEADER_SIZE, LOROM_BANK_SIZE};
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

    // TODO : take in account ROM mirrors :
    // https://problemkaputt.de/fullsnes.htm#snescarthirommappingromdividedinto64kbanksaround500games
    fn get_hirom_offset(&self, addr: SnesAddress) -> usize {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF => {
                let bank_index = (addr.bank & 0x3F) as usize;
                return bank_index * 0x10000 + addr.addr as usize;
            }
            _ => {
                Self::panic_invalid_addr(addr);
            }
        }
    }

    // TODO : take in account ROM mirrors :
    // https://problemkaputt.de/fullsnes.htm#snescartlorommappingromdividedinto32kbanksaround1500games
    fn get_lorom_offset(&self, addr: SnesAddress) -> usize {
        match addr.bank {
            0x00..=0x7D | 0x80..=0xFF => {
                if addr.addr >= 0x8000 {
                    let bank_index = (addr.bank & 0x7F) as usize;
                    let bank_offset = (addr.addr - 0x8000) as usize;
                    return bank_index * LOROM_BANK_SIZE + bank_offset;
                } else {
                    Self::panic_invalid_addr(addr);
                }
            }
            _ => {
                Self::panic_invalid_addr(addr);
            }
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

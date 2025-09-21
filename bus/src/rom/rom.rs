use crate::constants::{COPIER_HEADER_SIZE, LOROM_BANK_SIZE};
use crate::memory_region::MemoryRegion;
use crate::rom::error::RomError;
use crate::rom::mapping_mode::MappingMode;
use common::snes_address::SnesAddress;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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
    fn read(&self, addr: SnesAddress) -> u8 {
        let offset = self.to_offset(addr);

        return self.data.get(offset).copied().expect(&format!(
            "ERROR: Couldn't extract value from ROM at address: {:02X}{:04X}",
            addr.bank, addr.addr
        ));
    }

    fn write(&mut self, _addr: SnesAddress, _value: u8) {
        // ROM is read-only, ignore writes
        // TODO : Add a warning ?
    }
}

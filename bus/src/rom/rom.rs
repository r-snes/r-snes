use crate::constants::{
    COPIER_HEADER_SIZE, HEADER_SIZE, HIROM_HEADER_OFFSET, LOROM_BANK_SIZE, LOROM_HEADER_OFFSET,
};
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

impl MemoryRegion for Rom {
    // TODO : Check if mapping and mirroring is okay
    // (clearly not okay since switch to SnesAddress)
    fn read(&self, addr: SnesAddress) -> u8 {
        let offset = match self.map {
            MappingMode::HiRom => {
                // HiROM maps banks fully, 64 KiB per bank
                (addr.addr as u32 & 0x3FFFFF) as usize
            }
            MappingMode::LoRom => {
                // LoROM maps 32 KiB per bank, only $8000–$FFFF
                let bank = ((addr.bank as u32 >> 16) & 0x7F) as u32;
                let lo_offset = (addr.addr & 0x7FFF) as u32;
                (bank * (LOROM_BANK_SIZE as u32) + lo_offset) as usize
            }
            MappingMode::Unknown => {
                return 0xFF; // TODO : Crash here since shouldn't continue when MappingMode = unknown ?
            }
        };

        self.data.get(offset).copied().unwrap_or(0xFF)
    }

    fn write(&mut self, _addr: SnesAddress, _value: u8) {
        // ROM is read-only, ignore writes
        // TODO : Add a wawrning ?
    }
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
    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn print_rom_header(&self) {
        let header_offset = match self.map {
            MappingMode::LoRom => {
                println!("LoRom Mode");
                LOROM_HEADER_OFFSET
            }
            MappingMode::HiRom => {
                println!("hiRom Mode");
                HIROM_HEADER_OFFSET
            }
            MappingMode::Unknown => {
                println!("Cannot print ROM header: unknown ROM mapping.");
                return;
            }
        };

        if self.data.len() < header_offset + HEADER_SIZE {
            println!("ROM too small to contain a valid header.");
            return;
        }

        let header = &self.data[header_offset..header_offset + HEADER_SIZE];

        println!("\n--- ROM Header at offset 0x{:06X} ---", header_offset);
        Self::print_header_bytes(header);
        println!("-------------------------------------\n");
    }

    fn print_header_bytes(header: &[u8]) {
        let limit = HEADER_SIZE.min(header.len());

        for (i, chunk) in header[..limit].chunks(16).enumerate() {
            print!("{:04X}: ", i * 16);
            for byte in chunk {
                print!("{:02X} ", byte);
            }

            for _ in 0..(16 - chunk.len()) {
                print!("   ");
            }

            print!("| ");
            for byte in chunk {
                let c = if (0x20..=0x7E).contains(byte) {
                    *byte as char
                } else {
                    '.'
                };
                print!("{}", c);
            }
            println!();
        }
    }
}

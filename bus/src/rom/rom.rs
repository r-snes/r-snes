use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::error::RomError;
use super::mapping_mode::MappingMode;

pub struct Rom {
    pub data: Vec<u8>,
    pub map: MappingMode,
}

impl Rom {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, RomError> {
        let mut file = File::open(path).map_err(RomError::IoError)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(RomError::IoError)?;

        if buffer.len() < 0x8000 {
            return Err(RomError::FileTooSmall);
        }

        // Check for 512-byte header
        let rom_data = if buffer.len() % 0x8000 == 512 {
            buffer[512..].to_vec() // Remove useless "Copier" 512-byte header
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

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn print_rom_header(&self) {
        let header_offset = match self.map {
            MappingMode::LoRom => 0x7FC0,
            MappingMode::HiRom => 0xFFC0,
            MappingMode::Unknown => {
                println!("Cannot print ROM header: unknown ROM mapping.");
                return;
            }
        };

        if self.data.len() < header_offset + 64 {
            println!("ROM too small to contain a valid header.");
            return;
        }

        let header = &self.data[header_offset..header_offset + 64];

        println!("\n--- ROM Header at offset 0x{:06X} ---", header_offset);
        Self::print_header_bytes(header);
        println!("-------------------------------------\n");
    }

    fn print_header_bytes(header: &[u8]) {
        let limit = 64.min(header.len());

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

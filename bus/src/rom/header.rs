use crate::constants::{HEADER_SIZE, HIROM_HEADER_OFFSET, LOROM_HEADER_OFFSET};
use crate::rom::Rom;
use crate::rom::mapping_mode::MappingMode;

impl Rom {
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

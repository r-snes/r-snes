use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub struct Rom {
    pub data: Vec<u8>,
    pub map: RomMapping,
}

impl Rom {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // Check for 512-byte header
        let (rom_data, maybe_header) = if buffer.len() % 0x8000 == 512 {
            println!("512-byte header detected!");
            let header = &buffer[..512];
            (buffer[512..].to_vec(), Some(header))
        } else {
            println!("No header detected.");
            (buffer, None)
        };

        if let Some(header) = maybe_header {
            println!("\n--- Header Information ---");
            Self::print_header_info(header);
            println!("---------------------------\n");
        }

        Ok(Rom {
            data: rom_data,
            map: RomMapping::Unknown,
        })
    }

    fn print_header_info(header: &[u8]) {
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

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn read_byte(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }

    pub fn print_rom_header(&self) {
        let header_offset = match self.map {
            RomMapping::LoRom => 0x7FC0,
            RomMapping::HiRom => 0xFFC0,
            RomMapping::Unknown => {
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
        Self::print_header_info(header);
        println!("-------------------------------------\n");
    }
}

pub enum RomMapping {
    LoRom,
    HiRom,
    Unknown,
}

pub fn detect_rom_mapping(rom_data: &[u8]) -> RomMapping {
    if rom_data.len() < 0x10000 {
        return RomMapping::Unknown;
    }

    // Try LoROM header at 0x7FC0
    let lorom_score = score_header(rom_data, 0x7FC0);
    // Try HiROM header at 0xFFC0
    let hirom_score = score_header(rom_data, 0xFFC0);

    if lorom_score > hirom_score {
        RomMapping::LoRom
    } else if hirom_score > lorom_score {
        RomMapping::HiRom
    } else {
        RomMapping::Unknown
    }
}

fn score_header(rom_data: &[u8], header_offset: usize) -> u32 {
    if header_offset + 0x20 > rom_data.len() {
        return 0;
    }

    let mut score = 0;

    // Title should be mostly ASCII
    let title = &rom_data[header_offset..header_offset + 21];
    if title
        .iter()
        .all(|&c| (c == 0x20) || (0x20..=0x7E).contains(&c))
    {
        score += 1;
    }

    // Checksum and complement
    let checksum = read_u16(rom_data, header_offset + 0x1E);
    let checksum_complement = read_u16(rom_data, header_offset + 0x1C);

    if checksum != 0 && (checksum ^ checksum_complement) == 0xFFFF {
        score += 2;
    }

    score
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
}

use core::panic;

use crate::constants::{
    HEADER_CHECKSUM_COMPLEMENT_OFFSET, HEADER_CHECKSUM_OFFSET, HEADER_COUNTRY_OFFSET,
    HEADER_DEVELOPER_ID_OFFSET, HEADER_RAM_SIZE_OFFSET, HEADER_ROM_HARDWARE_OFFSET,
    HEADER_ROM_SIZE_OFFSET, HEADER_ROM_VERSION_OFFSET, HEADER_SIZE, HEADER_SPEED_MAP_OFFSET,
    HEADER_TITLE_LEN, HIROM_HEADER_OFFSET, LOROM_HEADER_OFFSET,
};
use crate::rom::Rom;
use crate::rom::mapping_mode::MappingMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomSpeed {
    Slow,
    Fast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartridgeHardware {
    RomOnly,
    RomRam,
    RomRamBattery,
    RomCoprocessor,
    RomCoprocessorRam,
    RomCoprocessorRamBattery,
    RomCoprocessorBattery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coprocessor {
    DSP(u8), // DSP-1, 2, 3, 4 → lower nibble
    GSU,     // SuperFX
    OBC1,
    SA1,
    SDD1,
    SRTC,
    Other,  // Super Game Boy / Satellaview
    Custom, // $Fx
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoStandard {
    NTSC,
    PAL,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    International, // 00h - any
    Japan,         // 00h - NTSC
    USA,           // 01h
    Europe,        // 02h
    Scandinavia,   // 03h
    Finland,       // 04h
    Denmark,       // 05h
    France,        // 06h
    Holland,       // 07h
    Spain,         // 08h
    Germany,       // 09h
    Italy,         // 0Ah
    China,         // 0Bh
    Indonesia,     // 0Ch
    SouthKorea,    // 0Dh
    Common,        // 0Eh
    Canada,        // 0Fh
    Brazil,        // 10h
    Australia,     // 11h
    OtherX,        // 12h
    OtherY,        // 13h
    OtherZ,        // 14h
}

#[derive(Debug, PartialEq)]
pub struct RomHeader {
    pub raw_bytes: [u8; HEADER_SIZE],
    pub title_bytes: [u8; HEADER_TITLE_LEN],
    pub rom_speed: RomSpeed,
    pub mapping_mode: MappingMode,
    pub hardware: CartridgeHardware,
    pub coprocessor: Option<Coprocessor>,
    pub rom_size: u8,
    pub ram_size: u8,
    pub country: Country,
    pub video_standard: VideoStandard,
    pub developer_id: u8,
    pub rom_version: u8,
    pub checksum_complement: u16,
    pub checksum: u16,
}

impl RomHeader {
    pub fn load_header(rom_data: &[u8], mapping_mode: MappingMode) -> RomHeader {
        let h_offset = mapping_mode.get_corresponding_header_offset();
        let slice = &rom_data[h_offset..h_offset + HEADER_SIZE];
        let header_bytes: [u8; HEADER_SIZE] = slice
            .try_into()
            .expect("ERROR: Couldn't extract the header from the ROM"); // Should be safe since multiple verification before

        let country = Self::read_country(header_bytes[HEADER_COUNTRY_OFFSET]);
        let rom_header = RomHeader {
            raw_bytes: (header_bytes),
            title_bytes: Self::read_title(header_bytes),
            rom_speed: Self::read_speed(header_bytes[HEADER_SPEED_MAP_OFFSET]),
            mapping_mode: Self::read_mapping(header_bytes[HEADER_SPEED_MAP_OFFSET]),
            hardware: Self::read_cartridge_hardware(header_bytes[HEADER_ROM_HARDWARE_OFFSET]),
            coprocessor: Self::read_coprocessor(header_bytes[HEADER_ROM_HARDWARE_OFFSET]),
            rom_size: header_bytes[HEADER_ROM_SIZE_OFFSET],
            ram_size: header_bytes[HEADER_RAM_SIZE_OFFSET],
            country: country,
            video_standard: Self::get_video_standard(country),
            developer_id: header_bytes[HEADER_DEVELOPER_ID_OFFSET],
            rom_version: header_bytes[HEADER_ROM_VERSION_OFFSET],
            checksum_complement: u16::from_be_bytes([
                header_bytes[HEADER_CHECKSUM_COMPLEMENT_OFFSET],
                header_bytes[HEADER_CHECKSUM_COMPLEMENT_OFFSET + 1],
            ]),
            checksum: u16::from_be_bytes([
                header_bytes[HEADER_CHECKSUM_OFFSET],
                header_bytes[HEADER_CHECKSUM_OFFSET + 1],
            ]),
        };

        rom_header
    }

    fn read_title(header_bytes: [u8; HEADER_SIZE]) -> [u8; HEADER_TITLE_LEN] {
        header_bytes[0..HEADER_TITLE_LEN]
            .try_into()
            .expect("ERROR: Couldn't extract the header from the ROM") // Should be safe since multiple verification before
    }

    fn read_speed(byte: u8) -> RomSpeed {
        let speed_bit = (byte >> 4) & 1;

        match speed_bit {
            // TODO : check if better way to represent a single bit
            0 => RomSpeed::Slow,
            1 => RomSpeed::Fast,
            _ => panic!("ERROR: Could not identify speed of ROM"),
        }
    }

    fn read_mapping(byte: u8) -> MappingMode {
        let map_value = byte & 0x0F;

        match map_value {
            // Commented values are mapping mode not currently implemented and rarely used
            0x0 => MappingMode::LoRom,
            0x1 => MappingMode::HiRom,
            // 0x2 => MappingMode::LoRomSdd1,
            // 0x3 => MappingMode::LoRomSa1,
            // 0x5 => MappingMode::ExHiRom,
            // 0xA => MappingMode::HiRomSpc7110,
            _ => panic!("ERROR: Could not identify mapping of ROM"),
        }
    }

    fn read_cartridge_hardware(byte: u8) -> CartridgeHardware {
        let hardware_value = byte & 0x0F;

        match hardware_value {
            0x0 => CartridgeHardware::RomOnly,
            0x1 => CartridgeHardware::RomRam,
            0x2 => CartridgeHardware::RomRamBattery,
            0x3 => CartridgeHardware::RomCoprocessor,
            0x4 => CartridgeHardware::RomCoprocessorRam,
            0x5 => CartridgeHardware::RomCoprocessorRamBattery,
            0x6 => CartridgeHardware::RomCoprocessorBattery,
            _ => panic!("ERROR: Could not identify hardware of ROM"),
        }
    }

    fn read_coprocessor(byte: u8) -> Option<Coprocessor> {
        let coprocessor = (byte & 0xF0) >> 4;

        match coprocessor {
            0x0 => Some(Coprocessor::DSP(1)),
            0x1 => Some(Coprocessor::GSU),
            0x2 => Some(Coprocessor::OBC1),
            0x3 => Some(Coprocessor::SA1),
            0x4 => Some(Coprocessor::SDD1),
            0x5 => Some(Coprocessor::SRTC),
            0xE => Some(Coprocessor::Other),
            0xF => Some(Coprocessor::Custom),
            _ => None,
        }
    }

    pub fn read_country(byte: u8) -> Country {
        match byte {
            0x00 => Country::Japan, // "0x00" sometimes means Japan or "International"
            0x01 => Country::USA,
            0x02 => Country::Europe,
            0x03 => Country::Scandinavia,
            0x04 => Country::Finland,
            0x05 => Country::Denmark,
            0x06 => Country::France,
            0x07 => Country::Holland,
            0x08 => Country::Spain,
            0x09 => Country::Germany,
            0x0A => Country::Italy,
            0x0B => Country::China,
            0x0C => Country::Indonesia,
            0x0D => Country::SouthKorea,
            0x0E => Country::Common,
            0x0F => Country::Canada,
            0x10 => Country::Brazil,
            0x11 => Country::Australia,
            0x12 => Country::OtherX,
            0x13 => Country::OtherY,
            0x14 => Country::OtherZ,
            _ => panic!("ERROR: Could not identify country of ROM"),
        }
    }

    pub fn get_video_standard(country: Country) -> VideoStandard {
        match country {
            Country::Japan
            | Country::USA
            | Country::SouthKorea
            | Country::Canada
            | Country::Brazil => VideoStandard::NTSC,

            Country::Europe
            | Country::Scandinavia
            | Country::Finland
            | Country::Denmark
            | Country::France
            | Country::Holland
            | Country::Spain
            | Country::Germany
            | Country::Italy
            | Country::China
            | Country::Indonesia
            | Country::Australia => VideoStandard::PAL,

            _ => VideoStandard::Other,
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::HIROM_BANK_SIZE;
    use crate::rom::test_rom::*;

    // #[test]
    // fn test_print_rom_header_hirom_with_title() {
    //     let data = create_valid_hirom(HIROM_BANK_SIZE);
    //     let rom = Rom {
    //         data: data,
    //         map: MappingMode::HiRom,
    //     };

    //     rom.print_rom_header();
    // }

    // #[test]
    // fn test_print_rom_header_hirom() {
    //     let data = vec![0; 0x10000];
    //     let rom = Rom {
    //         data: data,
    //         map: MappingMode::HiRom,
    //     };

    //     rom.print_rom_header();
    // }

    // #[test]
    // fn test_print_rom_header_lorom() {
    //     let data = vec![0; 0x10000];
    //     let rom = Rom {
    //         data: data,
    //         map: MappingMode::LoRom,
    //     };

    //     rom.print_rom_header();
    // }

    // #[test]
    // fn test_print_rom_header_lorom_too_small() {
    //     let data = vec![0; LOROM_HEADER_OFFSET];
    //     let rom = Rom {
    //         data: data,
    //         map: MappingMode::LoRom,
    //     };

    //     rom.print_rom_header();
    // }
}

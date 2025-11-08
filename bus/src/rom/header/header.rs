use std::fmt;

use crate::constants::{
    HEADER_CHECKSUM_COMPLEMENT_OFFSET, HEADER_CHECKSUM_OFFSET, HEADER_COUNTRY_OFFSET,
    HEADER_DEVELOPER_ID_OFFSET, HEADER_RAM_SIZE_OFFSET, HEADER_ROM_HARDWARE_OFFSET,
    HEADER_ROM_SIZE_OFFSET, HEADER_ROM_VERSION_OFFSET, HEADER_SIZE, HEADER_SPEED_MAP_OFFSET,
    HEADER_TITLE_LEN,
};
use crate::rom::header::cartridge_hardware::{CartridgeHardware, Coprocessor};
use crate::rom::header::country::{Country, VideoStandard};
use crate::rom::header::mapping_mode::MappingMode;
use crate::rom::header::rom_speed::RomSpeed;

#[derive(PartialEq)]
pub struct RomHeader {
    pub bytes: [u8; HEADER_SIZE],
    pub title: String,
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
        let country = Country::from_byte(header_bytes[HEADER_COUNTRY_OFFSET]);

        RomHeader {
            bytes: (header_bytes),
            title: Self::read_title(&header_bytes),
            rom_speed: RomSpeed::from_byte(header_bytes[HEADER_SPEED_MAP_OFFSET]),
            mapping_mode: MappingMode::from_byte(header_bytes[HEADER_SPEED_MAP_OFFSET]),
            hardware: CartridgeHardware::from_byte(header_bytes[HEADER_ROM_HARDWARE_OFFSET]),
            coprocessor: Coprocessor::from_byte(header_bytes[HEADER_ROM_HARDWARE_OFFSET]),
            rom_size: header_bytes[HEADER_ROM_SIZE_OFFSET],
            ram_size: header_bytes[HEADER_RAM_SIZE_OFFSET],
            country: country,
            video_standard: VideoStandard::from_country(country),
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
        }
    }

    fn read_title(header_bytes: &[u8; HEADER_SIZE]) -> String {
        // Convert to String and ignoring invalid UTF-8 safely
        String::from_utf8_lossy(&header_bytes[0..HEADER_TITLE_LEN]).to_string()
    }

    pub fn print_header_bytes(&self) {
        for chunk in self.bytes[..HEADER_SIZE].chunks(8) {
            for byte in chunk {
                print!("{:02X} ", byte);
            }
            println!();
        }
    }
}

impl fmt::Display for RomHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Title: '{}'\n", self.title)?;
        write!(f, "Rom Speed: {}\n", self.rom_speed)?;
        write!(f, "MappingMode: {}\n", self.mapping_mode)?;
        write!(f, "CartridgeHardware: {}\n", self.hardware)?;
        if let Some(coproc) = &self.coprocessor {
            write!(f, "Coprocessor: {}\n", coproc)?;
        } else {
            write!(f, "Coprocessor: None")?;
        }
        write!(f, "Rom size: {}\n", self.rom_size)?;
        write!(f, "Ram size: {}\n", self.ram_size)?;
        write!(f, "Country: {}\n", self.country)?;
        write!(f, "VideoStandard: {}\n", self.video_standard)?;
        write!(f, "Developer ID: {}\n", self.developer_id)?;
        write!(f, "Rom Version: {}\n", self.rom_version)?;
        write!(f, "Checksum Complement: {}\n", self.checksum_complement)?;
        write!(f, "Checksum: {}\n", self.checksum)
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

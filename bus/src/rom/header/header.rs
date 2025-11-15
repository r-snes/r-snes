use std::fmt;

use crate::constants::{
    HEADER_CHECKSUM_COMPLEMENT_OFFSET, HEADER_CHECKSUM_OFFSET, HEADER_COUNTRY_OFFSET,
    HEADER_DEVELOPER_ID_OFFSET, HEADER_RAM_SIZE_OFFSET, HEADER_ROM_HARDWARE_OFFSET,
    HEADER_ROM_SIZE_OFFSET, HEADER_ROM_VERSION_OFFSET, HEADER_SIZE, HEADER_SPEED_MAP_OFFSET,
    HEADER_TITLE_LEN,
};
use crate::rom::header::cartridge_hardware::CartridgeHardware;
use crate::rom::header::country::{Country, VideoStandard};
use crate::rom::header::mapping_mode::{MappingMode, RomSpeed, SpeedAndMappingMode};

/// Represents the header of a SNES ROM.
///
/// Contains all metadata extracted from the ROM header.
#[derive(PartialEq)]
pub struct RomHeader {
    pub bytes: [u8; HEADER_SIZE], // Raw bytes of the ROM header
    pub title: String,
    pub rom_speed: RomSpeed,         // ROM speed : fast or slow
    pub mapping_mode: MappingMode,   // Mapping mode specified in the header
    pub hardware: CartridgeHardware, // Type of hardware in cartridge (Coprocessor, RAM, etc...)
    pub rom_size: u8,
    pub ram_size: u8,
    pub country: Country,              // Country/region code of the ROM
    pub video_standard: VideoStandard, // based on the country (NTSC/PAL/Other)
    pub developer_id: u8,
    pub rom_version: u8,
    pub checksum_complement: u16,
    pub checksum: u16,
}

impl RomHeader {
    /// Loads a ROM header from an array of the ROM data and a specified mapping mode.
    ///
    /// Args:
    ///     rom_data: The full ROM data as a byte slice.
    ///     mapping_mode: Mapping mode used to locate the header.
    ///
    /// Returns:
    ///     A `RomHeader` struct populated with all extracted metadata.
    pub fn load_header(rom_data: &[u8], mapping_mode: MappingMode) -> RomHeader {
        let h_offset = mapping_mode.get_corresponding_header_offset();
        let slice = &rom_data[h_offset..h_offset + HEADER_SIZE];

        let header_bytes: [u8; HEADER_SIZE] = slice
            .try_into()
            .expect("ERROR: Couldn't extract the header from the ROM"); // Should be safe since multiple verification before
        let country = Country::from_byte(header_bytes[HEADER_COUNTRY_OFFSET]);
        let rom_speed_and_mapping_mode =
            SpeedAndMappingMode::from_byte(header_bytes[HEADER_SPEED_MAP_OFFSET]);

        RomHeader {
            bytes: header_bytes,
            title: String::from_utf8_lossy(&header_bytes[0..HEADER_TITLE_LEN]).to_string(),
            rom_speed: rom_speed_and_mapping_mode.rom_speed,
            mapping_mode: rom_speed_and_mapping_mode.mapping_mode,
            hardware: CartridgeHardware::from_byte(header_bytes[HEADER_ROM_HARDWARE_OFFSET]),
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

    /// Prints the raw header bytes to the console in hexadecimal format.
    ///
    /// Each line prints 8 bytes for readability.
    #[cfg(not(tarpaulin_include))]
    pub fn print_header_bytes(&self) {
        for chunk in self.bytes[..HEADER_SIZE].chunks(8) {
            for byte in chunk {
                print!("{:02X} ", byte);
            }
            println!();
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl fmt::Display for RomHeader {
    /// Formats the ROM header for display purposes.
    ///
    /// Prints all important metadata fields in a human-readable way.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Title: '{}'\n", self.title)?;
        write!(f, "Rom Speed: {}\n", self.rom_speed)?;
        write!(f, "MappingMode: {}\n", self.mapping_mode)?;
        write!(f, "CartridgeHardware: {}\n", self.hardware.layout)?;
        if let Some(coproc) = &self.hardware.coprocessor {
            write!(f, "Coprocessor: {}\n", coproc)?;
        } else {
            write!(f, "Coprocessor: None\n")?;
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
    use crate::{
        constants::HIROM_BANK_SIZE,
        rom::header::cartridge_hardware::{Coprocessor, HardwareLayout},
    };

    use super::*;
    use common::u16_split::*;

    fn create_custom_header() -> Vec<u8> {
        let mut header = vec![0u8; HEADER_SIZE];

        let title: &[u8; 21] = b"ABABABABABABABABABABA"; // 21 bytes
        header[0..21].copy_from_slice(title);

        header[21] = 0x10; // Fast ROM + LoRom
        header[22] = 0x23; // ROM + coprocessor (OBC1)
        header[23] = 0x08; // ROM size exponent (8 => 256 KB)
        header[24] = 0x00; // SRAM size (none)
        header[25] = 0x06; // Country (06 = France - PAL)
        header[26] = 0x01; // Licensee code (Nintendo standard)
        header[27] = 0x08; // Rom Version

        // Checksum
        let checksum: u16 = 0x0000;
        let complement: u16 = 0xFFFF;

        header[28] = *complement.lo();
        header[29] = *complement.hi();
        header[30] = *checksum.lo();
        header[31] = *checksum.hi();

        header
    }

    fn create_minimalist_rom(map: MappingMode) -> Vec<u8> {
        let mut fake_rom = vec![0; HIROM_BANK_SIZE];
        let header = create_custom_header();

        let header_offset = map.get_corresponding_header_offset();
        // let end = header_offset + header.len();

        // copy the header into the ROM at the proper offset
        fake_rom[header_offset..header_offset + HEADER_SIZE].copy_from_slice(&header);

        fake_rom
    }

    #[test]
    fn test_rom_header_creation() {
        let fake_rom = create_minimalist_rom(MappingMode::LoRom);
        let rom_header = RomHeader::load_header(&fake_rom, MappingMode::LoRom);

        assert_eq!(rom_header.bytes, *create_custom_header());
        assert_eq!(rom_header.title, "ABABABABABABABABABABA");
        assert_eq!(rom_header.rom_speed, RomSpeed::Fast);
        assert_eq!(rom_header.mapping_mode, MappingMode::LoRom);
        assert_eq!(rom_header.hardware.layout, HardwareLayout::RomCoprocessor);
        assert_eq!(rom_header.hardware.coprocessor, Some(Coprocessor::OBC1));
        assert_eq!(rom_header.rom_size, 8);
        assert_eq!(rom_header.ram_size, 0);
        assert_eq!(rom_header.country, Country::France);
        assert_eq!(rom_header.video_standard, VideoStandard::PAL);
        assert_eq!(rom_header.developer_id, 1);
        assert_eq!(rom_header.checksum_complement, 0xFFFF);
        assert_eq!(rom_header.checksum, 0x0000);
    }
}

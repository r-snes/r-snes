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

/// Represents the header of a SNES ROM.
///
/// Contains all metadata extracted from the ROM header.
#[derive(PartialEq)]
pub struct RomHeader {
    /// Raw bytes of the ROM header
    pub bytes: [u8; HEADER_SIZE],
    /// Game title
    pub title: String,
    /// ROM speed (fast/slow)
    pub rom_speed: RomSpeed,
    /// Mapping mode of the ROM
    pub mapping_mode: MappingMode,
    /// Type of cartridge hardware used by the ROM
    pub hardware: CartridgeHardware,
    /// Optional coprocessor present in the cartridge
    pub coprocessor: Option<Coprocessor>,
    /// Size of the ROM
    pub rom_size: u8,
    /// Size of the RAM
    pub ram_size: u8,
    /// Country/region code of the ROM
    pub country: Country,
    /// Video standard based on the country (NTSC/PAL/Other)
    pub video_standard: VideoStandard,
    /// Developer ID
    pub developer_id: u8,
    /// Version of the ROM
    pub rom_version: u8,
    /// Checksum complement of the ROM
    pub checksum_complement: u16,
    /// Checksum of the ROM
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

    /// Reads the title of the ROM from the header bytes.
    ///
    /// Args:
    ///     header_bytes: byte array containing the ROM header
    ///
    /// Returns:
    ///     The ROM title as a `String`, converting invalid UTF-8 safely.
    fn read_title(header_bytes: &[u8; HEADER_SIZE]) -> String {
        // Convert to String and ignoring invalid UTF-8 safely
        String::from_utf8_lossy(&header_bytes[0..HEADER_TITLE_LEN]).to_string()
    }

    /// Prints the raw header bytes to the console in hexadecimal format.
    ///
    /// Each line prints 8 bytes for readability.
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
    /// Formats the ROM header for display purposes.
    ///
    /// Prints all important metadata fields in a human-readable way.
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
mod tests {}

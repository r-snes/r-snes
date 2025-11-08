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
mod tests {
    use crate::constants::HIROM_BANK_SIZE;

    use super::*;
    use common::u16_split::*;

    fn create_custom_header() -> Vec<u8> {
        let mut header = vec![0u8; HEADER_SIZE];

        let title: &[u8; 21] = b"ABABABABABABABABABABA"; // 21 bytes
        header[0..21].copy_from_slice(title);

        header[21] = 0x00; // ROM Speed + Map Mode
        header[22] = 0x00; // Cartridge type (no co-processor)
        header[23] = 0x00; // ROM size exponent (8 => 256 KB)
        header[24] = 0x00; // SRAM size (none)
        header[25] = 0x00; // Country (01 = USA NTSC)
        header[26] = 0x00; // Licensee code (Nintendo standard)
        header[27] = 0x00; // Version (0 => original release)

        // Checksum (dummy values for now)
        let checksum: u16 = 0x0000;
        let complement: u16 = 0x0000;

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
    fn test_call_print_for_coverage() {
        let fake_rom = create_minimalist_rom(MappingMode::LoRom);
        let rom_header = RomHeader::load_header(&fake_rom, MappingMode::LoRom);

        rom_header.print_header_bytes();
    }

    #[test]
    fn test_rom_header_format() {
        let fake_rom = create_minimalist_rom(MappingMode::LoRom);
        let rom_header = RomHeader::load_header(&fake_rom, MappingMode::LoRom);
        let expected = "Title: 'ABABABABABABABABABABA'
Rom Speed: Slow
MappingMode: LoRom
CartridgeHardware: Rom
Coprocessor: DSP-1
Rom size: 0
Ram size: 0
Country: Japan
VideoStandard: NTSC
Developer ID: 0
Rom Version: 0
Checksum Complement: 0
Checksum: 0
";

        assert_eq!(format!("{}", rom_header), expected);
    }
}

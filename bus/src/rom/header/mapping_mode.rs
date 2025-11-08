use crate::constants::{
    HEADER_CHECKSUM_COMPLEMENT_OFFSET, HEADER_CHECKSUM_OFFSET, HEADER_MIN_LEN, HEADER_TITLE_LEN,
    HIROM_BANK_SIZE, HIROM_HEADER_OFFSET, LOROM_HEADER_OFFSET,
};
use std::{cmp::Ordering, fmt};

/// Represents the memory mapping mode of a SNES ROM.
///  
/// SNES cartridges can be mapped in different ways depending on how the
/// ROM is organized. This affects how the CPU addresses the ROM contents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MappingMode {
    /// LoROM (Low ROM) mapping mode.
    ///
    /// In LoROM, the CPU accesses the ROM in 32 KiB chunks starting at
    /// $8000 in each banks from $00–$7D and $80–$FF
    LoRom,

    /// HiROM (High ROM) mapping mode.
    ///
    /// In HiROM, the CPU can access the ROM in 64 KiB banks starting
    /// at $0000 of banks from $4–$7D and $C0–$FF.
    HiRom,
}

impl MappingMode {
    pub fn detect_rom_mapping(rom_data: &[u8]) -> Option<MappingMode> {
        if rom_data.len() < HIROM_BANK_SIZE {
            return None;
        }

        // Try LoROM header
        let lorom_score = Self::score_header(rom_data, LOROM_HEADER_OFFSET);
        // Try HiROM header
        let hirom_score = Self::score_header(rom_data, HIROM_HEADER_OFFSET);

        match lorom_score.cmp(&hirom_score) {
            Ordering::Greater => Some(MappingMode::LoRom),
            Ordering::Less => Some(MappingMode::HiRom),
            Ordering::Equal => None,
        }
    }

    pub fn get_corresponding_header_offset(&self) -> usize {
        match self {
            MappingMode::HiRom => HIROM_HEADER_OFFSET,
            MappingMode::LoRom => LOROM_HEADER_OFFSET,
        }
    }

    pub fn from_byte(byte: u8) -> MappingMode {
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

    fn score_header(rom_data: &[u8], header_offset: usize) -> u32 {
        if header_offset + HEADER_MIN_LEN > rom_data.len() {
            return 0;
        }

        let mut score = 0;

        // Title should be mostly ASCII
        let title = &rom_data[header_offset..header_offset + HEADER_TITLE_LEN];
        if title
            .iter()
            .all(|&c| (c == 0x20) || (0x20..=0x7E).contains(&c))
        {
            score += 1;
        }

        // Checksum and complement
        let checksum = u16::from_le_bytes([
            rom_data[header_offset + HEADER_CHECKSUM_OFFSET],
            rom_data[header_offset + HEADER_CHECKSUM_OFFSET + 1],
        ]);
        let checksum_complement = u16::from_le_bytes([
            rom_data[header_offset + HEADER_CHECKSUM_COMPLEMENT_OFFSET],
            rom_data[header_offset + HEADER_CHECKSUM_COMPLEMENT_OFFSET + 1],
        ]);

        if checksum != 0 && (checksum ^ checksum_complement) == 0xFFFF {
            score += 2;
        }

        score
    }
}

impl fmt::Display for MappingMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MappingMode::LoRom => write!(f, "LoRom"),
            MappingMode::HiRom => write!(f, "HiRom"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::test_rom::*;

    #[test]
    fn detect_lorom() {
        let rom = create_valid_lorom(HIROM_BANK_SIZE);
        let mode = MappingMode::detect_rom_mapping(&rom);

        assert_eq!(mode, Some(MappingMode::LoRom));
    }

    #[test]
    fn detect_hirom() {
        let rom = create_valid_hirom(HIROM_BANK_SIZE);
        let mode = MappingMode::detect_rom_mapping(&rom);

        assert_eq!(mode, Some(MappingMode::HiRom));
    }

    #[test]
    fn detect_unknown_if_too_small() {
        let rom = vec![0; HIROM_BANK_SIZE - 1];
        let mode = MappingMode::detect_rom_mapping(&rom);

        assert_eq!(mode, None);
    }

    #[test]
    fn unknown_empty_rom() {
        let rom = vec![0; HIROM_BANK_SIZE];
        let mode = MappingMode::detect_rom_mapping(&rom);

        assert_eq!(mode, None);
    }
}

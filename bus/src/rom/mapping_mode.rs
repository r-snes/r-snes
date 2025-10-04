use crate::constants::{
    HEADER_CHECKSUM_COMPLEMENT_OFFSET, HEADER_CHECKSUM_OFFSET, HEADER_MIN_LEN, HEADER_TITLE_LEN,
    HIROM_BANK_SIZE, HIROM_HEADER_OFFSET, LOROM_HEADER_OFFSET,
};
use std::cmp::Ordering;

#[derive(PartialEq, Debug)]
pub enum MappingMode {
    LoRom,
    HiRom,
    Unknown, // Error case, can't continue execution if we don't know mapping mode
}

impl MappingMode {
    pub fn detect_rom_mapping(rom_data: &[u8]) -> MappingMode {
        if rom_data.len() < HIROM_BANK_SIZE {
            return MappingMode::Unknown;
        }

        // Try LoROM header
        let lorom_score = Self::score_header(rom_data, LOROM_HEADER_OFFSET);
        // Try HiROM header
        let hirom_score = Self::score_header(rom_data, HIROM_HEADER_OFFSET);

        match lorom_score.cmp(&hirom_score) {
            Ordering::Greater => MappingMode::LoRom,
            Ordering::Less => MappingMode::HiRom,
            Ordering::Equal => MappingMode::Unknown,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a ROM buffer with a fake header at the given offset
    fn make_rom_with_header(offset: usize) -> Vec<u8> {
        let mut rom = vec![0; HIROM_BANK_SIZE]; // at least one HiROM bank

        let title = b"FAKE GAME TITLE      ";
        rom[offset..offset + HEADER_TITLE_LEN].copy_from_slice(title);

        let checksum: u16 = 0x1234;
        let complement: u16 = !checksum;

        rom[offset + HEADER_CHECKSUM_OFFSET] = (checksum & 0xFF) as u8;
        rom[offset + HEADER_CHECKSUM_OFFSET + 1] = (checksum >> 8) as u8;

        rom[offset + HEADER_CHECKSUM_COMPLEMENT_OFFSET] = (complement & 0xFF) as u8;
        rom[offset + HEADER_CHECKSUM_COMPLEMENT_OFFSET + 1] = (complement >> 8) as u8;

        rom
    }

    #[test]
    fn detect_lorom() {
        let rom = make_rom_with_header(LOROM_HEADER_OFFSET);
        let mode = MappingMode::detect_rom_mapping(&rom);

        assert_eq!(mode, MappingMode::LoRom);
    }

    #[test]
    fn detect_hirom() {
        let rom = make_rom_with_header(HIROM_HEADER_OFFSET);
        let mode = MappingMode::detect_rom_mapping(&rom);

        assert_eq!(mode, MappingMode::HiRom);
    }

    #[test]
    fn detect_unknown_if_too_small() {
        let rom = vec![0; HIROM_BANK_SIZE - 1];
        let mode = MappingMode::detect_rom_mapping(&rom);

        assert_eq!(mode, MappingMode::Unknown);
    }

    #[test]
    fn unknown_empty_rom() {
        let rom = vec![0; HIROM_BANK_SIZE];
        let mode = MappingMode::detect_rom_mapping(&rom);

        assert_eq!(mode, MappingMode::Unknown);
    }
}

use crate::constants::{
    HEADER_CHECKSUM_COMPLEMENT_OFFSET, HEADER_CHECKSUM_OFFSET, HEADER_MIN_LEN, HEADER_TITLE_LEN,
    HIROM_BANK_SIZE, HIROM_HEADER_OFFSET, LOROM_HEADER_OFFSET,
};
use std::cmp::Ordering;

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
        let checksum = Self::read_u16(rom_data, header_offset + HEADER_CHECKSUM_OFFSET);
        let checksum_complement =
            Self::read_u16(rom_data, header_offset + HEADER_CHECKSUM_COMPLEMENT_OFFSET);

        if checksum != 0 && (checksum ^ checksum_complement) == 0xFFFF {
            score += 2;
        }

        score
    }

    fn read_u16(data: &[u8], offset: usize) -> u16 {
        (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
    }
}

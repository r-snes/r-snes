pub enum RomMapping {
    LoRom,
    HiRom,
    Unknown,
}

impl RomMapping {
    pub fn detect_rom_mapping(rom_data: &[u8]) -> RomMapping {
        if rom_data.len() < 0x10000 {
            return RomMapping::Unknown;
        }

        // Try LoROM header at 0x7FC0
        let lorom_score = Self::score_header(rom_data, 0x7FC0);
        // Try HiROM header at 0xFFC0
        let hirom_score = Self::score_header(rom_data, 0xFFC0);

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
        let checksum = Self::read_u16(rom_data, header_offset + 0x1E);
        let checksum_complement = Self::read_u16(rom_data, header_offset + 0x1C);

        if checksum != 0 && (checksum ^ checksum_complement) == 0xFFFF {
            score += 2;
        }

        score
    }

    fn read_u16(data: &[u8], offset: usize) -> u16 {
        (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
    }
}

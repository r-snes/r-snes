//! OBC1: sprite (OAM) helper chip of Metal Combat: Falcon's Revenge
//!
//! The chip sits in front of the cartridge's 8 KiB S-RAM, mapped at
//! `$6000-$7FFF` in banks `$00-$3F` / `$80-$BF`. All of that window is plain
//! S-RAM, except a few registers near its end that let the game build an
//! OAM table (sprite attributes) in S-RAM by sprite number:
//!
//! - `$7FF5`: bit 0 selects the table: `0` at S-RAM `$1C00`, `1` at `$1800`
//! - `$7FF6`: sprite number (0-127)
//! - `$7FF0-$7FF3`: the 4 bytes of that sprite (X, Y, tile, attributes),
//!   at `table + number * 4`
//! - `$7FF4`: its 2 extra bits (X bit 8, size), packed 4 sprites per byte at
//!   `table + $200 + number / 4`; writes only change that sprite's 2 bits
//!
//! The registers are themselves stored in S-RAM (`$7FF5`-`$7FF7` read back
//! what was written), so the chip keeps no state of its own: the table and
//! sprite number are read back from S-RAM on every access.

use super::{CartridgeMemory, CoprocessorChip};
use crate::bus::AccessSpeed;
use common::snes_address::SnesAddress;

/// Offset of `$6000` in S-RAM: the window is 8 KiB
const WINDOW_MASK: u16 = 0x1FFF;
/// S-RAM offset of the table select register (`$7FF5`)
const BASE_SELECT: usize = 0x1FF5;
/// S-RAM offset of the sprite number register (`$7FF6`)
const SPRITE_NUMBER: usize = 0x1FF6;
/// Offset of the extra bits table, after the 128 * 4 bytes of the main table
const HIGH_TABLE: usize = 0x200;

/// OBC1 chip
#[derive(Debug, Default)]
pub struct Obc1;

/// Where an access in the OBC1 window lands
#[derive(Debug, PartialEq, Eq)]
enum Target {
    /// Byte `n` (0-3) of the selected sprite
    Sprite(usize),
    /// Extra bits of the selected sprite
    HighBits,
    /// Plain S-RAM (including the `$7FF5`-`$7FF7` registers)
    Ram,
}

impl Obc1 {
    /// Creates the chip
    pub fn new() -> Self {
        Self
    }

    /// S-RAM offset of the access, if `addr` is in the OBC1 window
    fn window_offset(addr: SnesAddress) -> Option<usize> {
        let in_banks = matches!(addr.bank & 0x7F, 0x00..=0x3F);
        let in_window = (0x6000..=0x7FFF).contains(&addr.addr);
        (in_banks && in_window).then_some((addr.addr & WINDOW_MASK) as usize)
    }

    fn target(offset: usize) -> Target {
        match offset {
            0x1FF0..=0x1FF3 => Target::Sprite(offset - 0x1FF0),
            0x1FF4 => Target::HighBits,
            _ => Target::Ram,
        }
    }

    /// Start of the selected OAM table in S-RAM
    fn table(mem: &CartridgeMemory) -> usize {
        match mem.sram.read(BASE_SELECT).unwrap_or(0) & 1 {
            0 => 0x1C00,
            _ => 0x1800,
        }
    }

    /// Selected sprite number (0-127)
    fn sprite(mem: &CartridgeMemory) -> usize {
        (mem.sram.read(SPRITE_NUMBER).unwrap_or(0) & 0x7F) as usize
    }

    /// S-RAM address of the extra bits byte of the selected sprite, and
    /// the position of its 2 bits in it
    fn high_bits(mem: &CartridgeMemory) -> (usize, u8) {
        let sprite = Self::sprite(mem);
        let address = Self::table(mem) + HIGH_TABLE + sprite / 4;
        (address, (sprite % 4) as u8 * 2)
    }
}

impl CoprocessorChip for Obc1 {
    fn read(&mut self, addr: SnesAddress, mem: CartridgeMemory) -> Option<(u8, AccessSpeed)> {
        let offset = Self::window_offset(addr)?;
        let address = match Self::target(offset) {
            Target::Sprite(n) => Self::table(&mem) + Self::sprite(&mem) * 4 + n,
            Target::HighBits => Self::high_bits(&mem).0,
            Target::Ram => offset,
        };
        // Without S-RAM nothing answers: open bus
        let value = mem.sram.read(address)?;
        Some((value, AccessSpeed::Slow))
    }

    fn write(&mut self, addr: SnesAddress, value: u8, mem: CartridgeMemory) -> Option<AccessSpeed> {
        let offset = Self::window_offset(addr)?;
        match Self::target(offset) {
            Target::Sprite(n) => {
                let address = Self::table(&mem) + Self::sprite(&mem) * 4 + n;
                mem.sram.write(address, value);
            }
            Target::HighBits => {
                let (address, shift) = Self::high_bits(&mem);
                let old = mem.sram.read(address).unwrap_or(0);
                let new = (old & !(0b11 << shift)) | ((value & 0b11) << shift);
                mem.sram.write(address, new);
            }
            Target::Ram => mem.sram.write(offset, value),
        }
        Some(AccessSpeed::Slow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::header::RomHeader;
    use crate::cartridge::header::mapping_mode::MappingMode;
    use crate::cartridge::sram::Sram;
    use crate::cartridge::test_rom::{create_valid_lorom, with_sram};
    use crate::constants::LOROM_HEADER_OFFSET;
    use common::snes_address::snes_addr;

    /// OBC1 with an 8 KiB S-RAM
    struct Board {
        chip: Obc1,
        sram: Sram,
    }

    impl Board {
        fn new() -> Self {
            let rom = with_sram(create_valid_lorom(0x10000), LOROM_HEADER_OFFSET, 0x03);
            let header = RomHeader::load_header(&rom, MappingMode::LoRom);
            let mut sram = Sram::new(&header);
            // known starting point instead of the $FF power-up value
            for i in 0..0x2000 {
                sram.write(i, 0);
            }
            Self {
                chip: Obc1::new(),
                sram,
            }
        }

        fn mem(&mut self) -> CartridgeMemory<'_> {
            CartridgeMemory {
                rom: &[],
                sram: &mut self.sram,
                map: MappingMode::LoRom,
            }
        }

        fn write(&mut self, addr: u16, value: u8) {
            let mem = CartridgeMemory {
                rom: &[],
                sram: &mut self.sram,
                map: MappingMode::LoRom,
            };
            assert!(self.chip.write(snes_addr!(0x00:addr), value, mem).is_some());
        }

        fn read(&mut self, addr: u16) -> u8 {
            let mem = CartridgeMemory {
                rom: &[],
                sram: &mut self.sram,
                map: MappingMode::LoRom,
            };
            self.chip.read(snes_addr!(0x00:addr), mem).unwrap().0
        }
    }

    #[test]
    fn test_plain_sram_in_the_window() {
        let mut board = Board::new();
        board.write(0x6000, 0x12);
        board.write(0x7FEF, 0x34);
        assert_eq!(board.read(0x6000), 0x12);
        assert_eq!(board.read(0x7FEF), 0x34);
        assert_eq!(board.sram.read(0x0000), Some(0x12));
        assert_eq!(board.sram.read(0x1FEF), Some(0x34));
    }

    #[test]
    fn test_sprite_bytes_go_to_the_selected_table() {
        let mut board = Board::new();
        board.write(0x7FF5, 0x00); // table at $1C00
        board.write(0x7FF6, 5); // sprite 5
        for (i, value) in [0x11, 0x22, 0x33, 0x44].into_iter().enumerate() {
            board.write(0x7FF0 + i as u16, value);
        }
        assert_eq!(board.sram.read(0x1C00 + 5 * 4), Some(0x11));
        assert_eq!(board.sram.read(0x1C00 + 5 * 4 + 3), Some(0x44));
        assert_eq!(board.read(0x7FF2), 0x33);

        board.write(0x7FF5, 0x01); // table at $1800
        board.write(0x7FF0, 0x55);
        assert_eq!(board.sram.read(0x1800 + 5 * 4), Some(0x55));
        // the other table is untouched
        assert_eq!(board.sram.read(0x1C00 + 5 * 4), Some(0x11));
    }

    #[test]
    fn test_high_bits_only_change_the_selected_sprite() {
        let mut board = Board::new();
        board.write(0x7FF5, 0x00);
        board.sram.write(0x1C00 + 0x200 + 1, 0b1111_1111);

        // sprite 6: 2nd byte of the high table, bits 4-5
        board.write(0x7FF6, 6);
        board.write(0x7FF4, 0b1111_1101); // only the low 2 bits count
        assert_eq!(board.sram.read(0x1C00 + 0x200 + 1), Some(0b1101_1111));
        assert_eq!(board.read(0x7FF4), 0b1101_1111);
    }

    #[test]
    fn test_registers_read_back_from_sram() {
        let mut board = Board::new();
        board.write(0x7FF5, 0x01);
        board.write(0x7FF6, 0xFF);
        board.write(0x7FF7, 0x42);
        assert_eq!(board.read(0x7FF5), 0x01);
        assert_eq!(board.read(0x7FF6), 0xFF);
        assert_eq!(board.read(0x7FF7), 0x42);
        // sprite number uses 7 bits: $FF selects sprite 127
        board.write(0x7FF0, 0x99);
        assert_eq!(board.sram.read(0x1800 + 127 * 4), Some(0x99));
    }

    #[test]
    fn test_window_and_banks() {
        let mut board = Board::new();
        let mut chip = Obc1::new();
        assert!(chip.read(snes_addr!(0xBF:0x7FF0), board.mem()).is_some());
        assert!(chip.read(snes_addr!(0x40:0x6000), board.mem()).is_none());
        assert!(chip.read(snes_addr!(0x00:0x5FFF), board.mem()).is_none());
        assert!(chip.read(snes_addr!(0x00:0x8000), board.mem()).is_none());
    }
}

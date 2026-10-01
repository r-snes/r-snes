//! Constants describing the SNES memory layout, the ROM header format
//! and the controller timings

/// Size of a memory bank (64 KiB)
pub const BANK_SIZE: usize = 0xFFFF + 1;

/// First address of the I/O zone within a bank
pub const IO_START_ADDRESS: u16 = 0x2000;
/// Last address of the I/O zone within a bank
pub const IO_END_ADDRESS: u16 = 0x5FFF;
/// Size of the I/O zone (equal to 0x4000)
pub const IO_SIZE: usize = (IO_END_ADDRESS - IO_START_ADDRESS + 1) as usize;
const _: () = assert!(IO_SIZE == 0x4000);

/// Number of banks the WRAM spans on
pub const WRAM_BANK_NB: usize = 2;
/// Total size of the WRAM (128 KiB)
pub const WRAM_SIZE: usize = BANK_SIZE * WRAM_BANK_NB;
/// Largest S-RAM size exponent accepted from the ROM header (`1024 << 7` == 128 KiB)
pub const SRAM_MAX_SIZE_EXP: u8 = 7;

/// Length of the game title at the start of the header
pub const HEADER_TITLE_LEN: usize = 21;
/// Offset of the header in a LoROM ROM
pub const LOROM_HEADER_OFFSET: usize = 0x7FC0;
/// Offset of the header in a HiROM ROM
pub const HIROM_HEADER_OFFSET: usize = 0xFFC0;
/// Offset in the header of the ROM speed and mapping mode byte
pub const HEADER_SPEED_MAP_OFFSET: usize = 0x15;
/// Offset in the header of the cartridge hardware byte
pub const HEADER_ROM_HARDWARE_OFFSET: usize = 0x16;
/// Offset in the header of the ROM size byte
pub const HEADER_ROM_SIZE_OFFSET: usize = 0x17;
/// Offset in the header of the RAM size byte
pub const HEADER_RAM_SIZE_OFFSET: usize = 0x18;
/// Offset in the header of the country byte
pub const HEADER_COUNTRY_OFFSET: usize = 0x19;
/// Offset in the header of the developer ID byte
pub const HEADER_DEVELOPER_ID_OFFSET: usize = 0x1A;
/// Offset in the header of the ROM version byte
pub const HEADER_ROM_VERSION_OFFSET: usize = 0x1B;
/// Offset in the header of the checksum complement (2 bytes)
pub const HEADER_CHECKSUM_COMPLEMENT_OFFSET: usize = 0x1C;
/// Offset in the header of the checksum (2 bytes)
pub const HEADER_CHECKSUM_OFFSET: usize = 0x1E;

/// Minimum number of bytes needed for scoring
pub const HEADER_MIN_LEN: usize = 0x20;
/// Size of the header (equal to 0x40)
pub const HEADER_SIZE: usize = 64;

/// Size of the ROM chunk mapped in each bank in LoROM (32 KiB)
pub const LOROM_BANK_SIZE: usize = 0x8000;
/// Size of the ROM chunk mapped in each bank in HiROM (64 KiB)
pub const HIROM_BANK_SIZE: usize = 0xFFFF + 1;
/// Size of the optional copier header at the start of some ROM files
pub const COPIER_HEADER_SIZE: usize = 512;

/// Delay between V-Blank start and the auto-read strobe (~74.5 dots).
pub const AUTO_JOYPAD_START_DELAY: u32 = 298;
const _: () = assert!(AUTO_JOYPAD_START_DELAY as f64 / 4_f64 == 74.5);
/// Total duration of the auto-read, during which HVBJOY bit 0 is set.
pub const AUTO_JOYPAD_READ_CYCLES: u32 = 4224;
/// Master cycles per serial bit (16 bits spread evenly over the read).
pub const AUTO_JOYPAD_BIT_CYCLES: u32 = AUTO_JOYPAD_READ_CYCLES / 16;
const _: () = assert!(AUTO_JOYPAD_READ_CYCLES.is_multiple_of(16));
/// JOYOUT bit driving the latch line of the controller ports
pub const JOYOUT_LATCH: u8 = 1 << 0;

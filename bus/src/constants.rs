// Generic constants
pub const BANK_SIZE: usize = 0xFFFF + 1; // 64 KiB per bank

// IO Memory zone
pub const IO_START_ADDRESS: u16 = 0x2000;
pub const IO_END_ADDRESS: u16 = 0x5FFF;
pub const IO_SIZE: usize = (IO_END_ADDRESS - IO_START_ADDRESS + 1) as usize; // Equal to 0X4000

// WRAM Memory zone
pub const WRAM_BANK_NB: usize = 2; // WRAM spans on 2 banks
pub const WRAM_SIZE: usize = BANK_SIZE * WRAM_BANK_NB;

// ROM Header
pub const LOROM_HEADER_OFFSET: usize = 0x7FC0;
pub const HIROM_HEADER_OFFSET: usize = 0xFFC0;
pub const HEADER_TITLE_LEN: usize = 21;
pub const HEADER_CHECKSUM_OFFSET: usize = 0x1E;
pub const HEADER_CHECKSUM_COMPLEMENT_OFFSET: usize = 0x1C;
pub const HEADER_MIN_LEN: usize = 0x20; // Minimum number of bytes needed for scoring
pub const HEADER_SIZE: usize = 64;

// ROM Memory Zone
pub const LOROM_BANK_SIZE: usize = 0x8000; // 32 KiB
pub const HIROM_BANK_SIZE: usize = 0xFFFF + 1; // 64 KiB
pub const COPIER_HEADER_SIZE: usize = 512; // Optional copier header

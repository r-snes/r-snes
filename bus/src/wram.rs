use crate::memory_region::MemoryRegion;

const BANK_SIZE: usize = 0xFFFF + 1; // 64 KiB per bank
const BANK_COUNT: usize = 2; // WRAM spans 2 banks
const SIZE: usize = BANK_SIZE * BANK_COUNT;
const MIRROR_MASK: u32 = 0x1_FFFF; // 17 bits : wraps addresses

pub struct Wram {
    data: [u8; SIZE], // 128 KiB WRAM
}

impl Wram {
    pub fn new() -> Self {
        Self { data: [0; SIZE] }
    }

    fn map_addr(addr: u32) -> usize {
        (addr & MIRROR_MASK) as usize
    }
}

impl MemoryRegion for Wram {
    fn read(&self, addr: u32) -> u8 {
        let offset = Self::map_addr(addr);
        self.data.get(offset).copied().unwrap_or(0xFF)
    }

    fn write(&mut self, addr: u32, value: u8) {
        let offset = Self::map_addr(addr);
        if offset < self.data.len() {
            self.data[offset] = value;
        }
    }
}

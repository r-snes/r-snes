use crate::constants::{MIRROR_MASK, WRAM_SIZE};
use crate::memory_region::MemoryRegion;

pub struct Wram {
    data: [u8; WRAM_SIZE], // 128 KiB WRAM
}

impl Wram {
    pub fn new() -> Self {
        Self {
            data: [0; WRAM_SIZE],
        }
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

use crate::memory_region::MemoryRegion;

pub struct Wram {
    data: [u8; 128 * 1024], // 128 KiB WRAM
}

impl Wram {
    pub fn new() -> Self {
        Self {
            data: [0; 128 * 1024],
        }
    }
}

impl MemoryRegion for Wram {
    fn read(&self, addr: u32) -> u8 {
        let offset = (addr & 0x1FFFF) as usize; // Wraps within 128KiB
        self.data.get(offset).copied().unwrap_or(0xFF)
    }

    fn write(&mut self, addr: u32, value: u8) {
        let offset = (addr & 0x1FFFF) as usize;
        if offset < self.data.len() {
            self.data[offset] = value;
        }
    }
}

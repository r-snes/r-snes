use crate::memory_region::MemoryRegion;

const IO_START: u32 = 0x2000;
const IO_END: u32 = 0x5FFF;
const IO_SIZE: usize = (IO_END - IO_START + 1) as usize;

pub struct Io {
    // TODO : Implement real CPU, PPU, APU, etc... memories.
    // This memory is only 0x4000 long because all of the IO is mirrored in all banks from 0x00/0X3F - 0X80/0XBF
    memory: [u8; IO_SIZE],
}

impl Io {
    pub fn new() -> Self {
        Self {
            memory: [0; IO_SIZE],
        }
    }

    fn map_offset(addr: u32) -> Option<usize> {
        let offset_in_bank = addr & 0xFFFF; // address within the current 64 KiB bank

        if (IO_START..=IO_END).contains(&offset_in_bank) {
            let index = (offset_in_bank - IO_START) as usize;
            Some(index)
        } else {
            None
        }
    }
}

impl MemoryRegion for Io {
    fn read(&self, addr: u32) -> u8 {
        if let Some(offset) = Self::map_offset(addr) {
            self.memory[offset]
        } else {
            0xFF // TODO: open bus
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        if let Some(offset) = Self::map_offset(addr) {
            self.memory[offset] = value;
        }
    }
}

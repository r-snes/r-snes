pub const VRAM_SIZE: usize = 64 * 1024; // 64 KB

pub struct PPU {
    vram: [u8; VRAM_SIZE],
}

impl PPU {
    pub fn new() -> Self {
        Self {
            vram: [0; VRAM_SIZE],
        }
    }

    pub fn write_vram(&mut self, addr: usize, value: u8) {
        if addr >= VRAM_SIZE {
            eprintln!("PPU: can't write to 0x{:04X} (invalid address)", addr);
            return;
        }

        self.vram[addr] = value;
    }

    pub fn read_vram(&self, addr: usize) -> u8 {
        if addr >= VRAM_SIZE {
            eprintln!("PPU: can't read from 0x{:04X} (invalid address)", addr);
            return 0;
        }

        return self.vram[addr];
    }
}

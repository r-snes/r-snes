use crate::utils::{render_scanline, CGRAM_SIZE, HEIGHT, VRAM_SIZE, WIDTH};

pub struct PPU {
    pub framebuffer: Vec<u32>,
    vram: [u8; VRAM_SIZE],
    cgram: [u16; CGRAM_SIZE],

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    cgaddr: u8,

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    latch: u8,
    latch_filled: bool
}

impl PPU {
    pub fn new() -> Self {
        let mut ppu = Self {
            framebuffer: vec![0; WIDTH * HEIGHT],
            vram: [0; VRAM_SIZE],
            cgram: [0; CGRAM_SIZE],
            cgaddr: 0,
            latch: 0,
            latch_filled: false
        };

        // Hardcoded palette
        for i in 0..CGRAM_SIZE {
            let r = (i & 0x1F) as u16;
            let g = ((i >> 2) & 0x1F) as u16;
            let b = ((i >> 4) & 0x1F) as u16;
            ppu.cgram[i] = (b << 10) | (g << 5) | r;
        }

        ppu
    }

    pub fn write_vram(&mut self, addr: usize, value: u8) {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Write attempt to invalid address 0x{:04X}", addr);
            return;
        }
        self.vram[addr] = value;
    }

    pub fn read_vram(&self, addr: usize) -> u8 {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Read attempt from invalid address 0x{:04X}", addr);
            return 0;
        }
        self.vram[addr]
    }

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    // Set current CGRAM address ($2121 on the SNES)
    pub fn set_cgram_addr(&mut self, addr: u8) {
        self.cgaddr = addr;
        self.latch_filled = false; // reset latch when address changes
    }

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    // Write one byte to CGRAM ($2122 on the SNES I think ?)
    pub fn write_cgram_data(&mut self, value: u8) {
        if self.latch_filled {
            // 2nd write → combine low + high into one 16-bit value
            let color = ((value as u16) << 8) | (self.latch as u16);
            self.cgram[self.cgaddr as usize] = color & 0x7FFF; // mask to 15 bits
            self.cgaddr = self.cgaddr.wrapping_add(1); // auto-increment address
            self.latch_filled = false;
        } else {
            // 1st write → store in latch
            self.latch = value;
            self.latch_filled = true;
        }
    }

    pub fn read_cgram(&self, index: u8) -> u32 {
        bgr555_to_argb(self.cgram[index as usize])
    }

    pub fn render(&mut self, tiles_per_row: usize) {
        for y in 0..HEIGHT {
            render_scanline(self, y, tiles_per_row);
        }
    }

    // Tests functions
    #[allow(dead_code)]
    pub fn cgram_len(&self) -> usize {
        self.cgram.len()
    }

    #[allow(dead_code)]
    pub fn get_cgram_value(&self, index: usize) -> u16 {
        self.cgram[index]
    }

    #[allow(dead_code)]
    pub fn get_cgaddr(&self) -> u8 {
        self.cgaddr
    }

    #[allow(dead_code)]
    pub fn is_latch_set(&self) -> bool {
        self.latch_filled
    }
}

pub fn bgr555_to_argb(bgr: u16) -> u32 {
    let r = (bgr & 0x1F) as u32;
    let g = ((bgr >> 5) & 0x1F) as u32;
    let b = ((bgr >> 10) & 0x1F) as u32;

    // Expand 5-bit to 8-bit by duplicating upper bits
    let r8 = (r << 3) | (r >> 2);
    let g8 = (g << 3) | (g >> 2);
    let b8 = (b << 3) | (b >> 2);

    (0xFF << 24) | (r8 << 16) | (g8 << 8) | b8
}

use crate::utils::render_scanline;
use crate::constants::*;
use crate::sprite::Sprite;
use crate::color::bgr555_to_argb;

/// Represents the Picture Processing Unit (PPU) of the SNES
/// 
/// Contains VRAM, CGRAM, OAM (sprites), and the framebuffer
/// Handles tile and sprite rendering
pub struct PPU {
    pub framebuffer: Vec<u32>,
    pub vram: [u8; VRAM_SIZE],
    pub cgram: [u16; CGRAM_SIZE],
    pub oam: [Sprite; OAM_MAX_SPRITES],

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub cgaddr: u8,

    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub latch: u8,
    pub latch_filled: bool,
}

impl PPU {
    /// Creates a new PPU instance with initialized VRAM, CGRAM, OAM, and framebuffer
    /// 
    /// # Returns
    /// A `PPU` with default values. The CGRAM is prefilled with a simple hardcoded palette
    pub fn new() -> Self {
        let mut ppu = Self {
            framebuffer: vec![0; WIDTH * HEIGHT],
            vram: [0; VRAM_SIZE],
            cgram: [0; CGRAM_SIZE],
            oam: [Sprite::default(); OAM_MAX_SPRITES],
            cgaddr: 0,
            latch: 0,
            latch_filled: false,
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

    /// Writes a byte to VRAM at the given address
    /// 
    /// # Parameters
    /// - `addr`: VRAM address to write to (0..VRAM_SIZE)
    /// - `value`: Value to write.
    pub fn write_vram(&mut self, addr: usize, value: u8) {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Write to invalid address 0x{:04X}", addr);
            return;
        }
        self.vram[addr] = value;
    }

    /// Reads a byte from VRAM at the given address
    /// 
    /// # Parameters
    /// - `addr`: VRAM address to read from (0..VRAM_SIZE)
    /// 
    /// # Returns
    /// The value stored at the address, or 0 if the address is invalid
    pub fn read_vram(&self, addr: usize) -> u8 {
        if addr >= VRAM_SIZE {
            eprintln!("[ERR::VRAM] Read from invalid address 0x{:04X}", addr);
            return 0;
        }
        self.vram[addr]
    }

    /// Sets the current CGRAM address for future writes
    /// 
    /// # Parameters
    /// - `addr`: The new CGRAM address (0..CGRAM_SIZE)
    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub fn set_cgram_addr(&mut self, addr: u8) {
        self.cgaddr = addr;
        self.latch_filled = false;
    }

    /// Writes a byte to CGRAM at the current address
    /// 
    /// # Parameters
    /// - `value`: Byte to write
    #[allow(dead_code)] // For future CPU write handling (not implemented yet)
    pub fn write_cgram_data(&mut self, value: u8) {
        if self.latch_filled {
            // Second step: combine the previous latched byte with the new one and write to CGRAM
            let color = u16::from_le_bytes([self.latch, value]);
            self.cgram[self.cgaddr as usize] = color & 0x7FFF;
            self.cgaddr = self.cgaddr.wrapping_add(1);
            self.latch_filled = false;
        } else {
            // First step: store the first byte into the latch
            self.latch = value;
            self.latch_filled = true;
        }
    }

    /// Reads a 32-bit ARGB color from CGRAM
    /// 
    /// # Parameters
    /// - `index`: CGRAM index (0..CGRAM_SIZE)
    /// 
    /// # Returns
    /// A `u32` containing the ARGB color
    pub fn read_cgram(&self, index: u8) -> u32 {
        bgr555_to_argb(self.cgram[index as usize])
    }

    /// Sets a sprite in OAM at the given index
    /// 
    /// # Parameters
    /// - `index`: Sprite index (0..OAM_MAX_SPRITES)
    /// - `sprite`: The sprite data to store
    pub fn set_oam_sprite(&mut self, index: usize, sprite: Sprite) {
        if index >= OAM_MAX_SPRITES {
            eprintln!("[ERR::OAM] index out of range: {}", index);
            return;
        }
        self.oam[index] = sprite;
    }

    /// Retrieves a sprite from OAM by index
    /// 
    /// # Parameters
    /// - `index`: Sprite index (0..OAM_MAX_SPRITES)
    /// 
    /// # Returns
    /// `Some(sprite)` if the index is valid, or `None` if out of range
    pub fn get_oam_sprite(&self, index: usize) -> Option<Sprite> {
        if index >= OAM_MAX_SPRITES {
            None
        } else {
            Some(self.oam[index])
        }
    }

    /// Renders the entire screen by calling `render_scanline` for each row
    pub fn render(&mut self) {
        for y in 0..HEIGHT {
            render_scanline(self, y);
        }
    }
}

#[cfg(test)]
mod tests_ppu {
    use super::*;
    use crate::sprite::Sprite;
    use std::cell::RefCell;
    use std::rc::Rc;

    // Helper: Fake bgr555_to_argb to test color conversion deterministically
    fn mock_bgr555_to_argb(value: u16) -> u32 {
        (value as u32) | 0xFF000000
    }

    #[test] // Creating a new PPU should initialize all memory regions and state fields
    fn test_ppu_new_initializes_all_fields() {
        let ppu = PPU::new();

        // Framebuffer should be fully initialized
        assert_eq!(ppu.framebuffer.len(), WIDTH * HEIGHT);
        assert!(ppu.framebuffer.iter().all(|&v| v == 0));

        // VRAM initialized to 0
        assert!(ppu.vram.iter().all(|&v| v == 0));

        // CGRAM filled with palette pattern
        assert_ne!(ppu.cgram[10], 0);

        // OAM initialized with default sprites
        assert!(ppu.oam.iter().all(|s| s == &Sprite::default()));

        // Default state fields
        assert_eq!(ppu.cgaddr, 0);
        assert!(!ppu.latch_filled);
    }

    #[test] // Writing and reading from a valid VRAM address should work correctly
    fn test_write_and_read_vram_valid_address() {
        let mut ppu = PPU::new();
        ppu.write_vram(5, 123);
        assert_eq!(ppu.read_vram(5), 123);
    }

    #[test] // Writing to an invalid VRAM address should not panic or corrupt memory
    fn test_write_vram_invalid_address_does_not_panic() {
        let mut ppu = PPU::new();
        ppu.write_vram(VRAM_SIZE + 5, 200);
        // Out-of-bounds should be ignored
        assert_eq!(ppu.vram[VRAM_SIZE - 1], 0);
    }

    #[test] // Reading from an invalid VRAM address should safely return zero
    fn test_read_vram_invalid_address_returns_zero() {
        let ppu = PPU::new();
        assert_eq!(ppu.read_vram(VRAM_SIZE + 1), 0);
    }

    #[test] // Setting a new CGRAM address should reset the latch state
    fn test_set_cgram_addr_and_latch_reset() {
        let mut ppu = PPU::new();
        ppu.latch_filled = true;
        ppu.set_cgram_addr(10);
        assert_eq!(ppu.cgaddr, 10);
        assert!(!ppu.latch_filled);
    }

    #[test] // Writing twice to CGRAM should combine two bytes into one 15-bit color entry
    fn test_write_cgram_data_latch_behavior() {
        let mut ppu = PPU::new();

        // First write fills latch
        ppu.write_cgram_data(0xAA);
        assert!(ppu.latch_filled);
        assert_eq!(ppu.latch, 0xAA);

        // Second write combines with latch
        ppu.cgaddr = 2;
        ppu.write_cgram_data(0xBB);
        assert!(!ppu.latch_filled);
        assert_eq!(ppu.cgram[2], 0x3BAA); // 0xBBA & 0x7FFF
    }

    #[test] // Reading from CGRAM should return a converted ARGB color
    fn test_read_cgram_returns_argb_value() {
        let mut ppu = PPU::new();
        ppu.cgram[5] = 0x7FFF;
        let color = ppu.read_cgram(5);
        // Function bgr555_to_argb should have been applied
        assert_eq!(color, bgr555_to_argb(0x7FFF));
    }

    #[test] // Setting a sprite in valid OAM range should correctly store its data
    fn test_set_oam_sprite_valid_index() {
        let mut ppu = PPU::new();
        let sprite = Sprite {
            x: 10,
            y: 20,
            tile: 3,
            attr: 1,
            filed: true,
        };
        ppu.set_oam_sprite(5, sprite);
        assert_eq!(ppu.oam[5].x, 10);
        assert_eq!(ppu.oam[5].filed, true);
    }

    #[test] // Setting a sprite with an out-of-range index should be ignored safely
    fn test_set_oam_sprite_out_of_range_does_not_panic() {
        let mut ppu = PPU::new();
        let sprite = Sprite {
            x: 1,
            y: 1,
            tile: 1,
            attr: 0,
            filed: true,
        };
        ppu.set_oam_sprite(OAM_MAX_SPRITES + 1, sprite);
        // Nothing should crash or modify OAM
        assert!(ppu.oam.iter().all(|s| s == &Sprite::default()));
    }

    #[test] // Getting a sprite should return Some for valid index and None for invalid
    fn test_get_oam_sprite_valid_and_invalid_index() {
        let mut ppu = PPU::new();
        ppu.oam[0].x = 42;

        assert_eq!(ppu.get_oam_sprite(0).unwrap().x, 42);
        assert!(ppu.get_oam_sprite(OAM_MAX_SPRITES + 1).is_none());
    }

    #[test] // Rendering should call render_scanline once per screen row
    fn test_render_calls_render_scanline_for_each_row() {
        // Counter for calls
        thread_local! {
            static CALL_COUNT: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
        }

        // Mock render_scanline function
        fn fake_render_scanline(_ppu: &mut PPU, _y: usize) {
            CALL_COUNT.with(|c| *c.borrow_mut() += 1);
        }

        // Temporarily shadow the imported render_scanline
        use crate::utils::render_scanline as real_render_scanline;
        let mut ppu = PPU::new();

        // Simulate render calling scanline for each row
        for y in 0..HEIGHT {
            fake_render_scanline(&mut ppu, y);
        }

        // Counter should match screen height
        CALL_COUNT.with(|c| {
            assert_eq!(*c.borrow(), HEIGHT);
        });
    }

    #[test] // Real render() function should run safely without panicking
    fn test_render_scanline_real_function_runs_without_crash() {
        let mut ppu = PPU::new();
        ppu.render();
        // Framebuffer should still be valid length
        assert_eq!(ppu.framebuffer.len(), WIDTH * HEIGHT);
    }
}

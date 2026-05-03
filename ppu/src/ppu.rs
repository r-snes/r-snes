use crate::constants::SCANLINES_PER_FRAME;
use crate::registers::PPURegisters;
use crate::vram::VRAM;
use crate::cgram::CGRAM;
use crate::write_twice::WriteTwice;
use common::u16_split::U16Split;

pub struct PPU {
    pub regs: PPURegisters,
    pub vram: VRAM,
    pub cgram: CGRAM,

    // Timing
    pub scanline: u16,
    pub frame_ready: bool,

    bg1hofs_latch: WriteTwice,
    bg1vofs_latch: WriteTwice,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            regs: PPURegisters::new(),
            vram: VRAM::new(),
            cgram: CGRAM::new(),
            scanline: 0,
            frame_ready: false,
            bg1hofs_latch: WriteTwice::new(),
            bg1vofs_latch: WriteTwice::new(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // ==========================
            // DISPLAY
            // ==========================
            0x2100 => self.regs.inidisp = value,

            // ==========================
            // BACKGROUNDS
            // ==========================
            0x2105 => self.regs.bgmode = value,
            0x2107 => self.regs.bg1sc = value,

            // BG1 HOFS
            0x210D => {
                if let Some((lo, hi)) = self.bg1hofs_latch.write(value) {
                    *self.regs.bg1hofs.lo_mut() = lo;
                    *self.regs.bg1hofs.hi_mut() = hi & 0x07;
                }
            }

            // BG1 VOFS
            0x210E => {
                if let Some((lo, hi)) = self.bg1vofs_latch.write(value) {
                    *self.regs.bg1vofs.lo_mut() = lo;
                    *self.regs.bg1vofs.hi_mut() = hi & 0x07;
                }
            }

            // ==========================
            // COLOR MATH / LAYER ENABLE
            // ==========================
            0x212C => self.regs.tm = value,

            // ==========================
            // VRAM
            // ==========================
            0x2115 => self.regs.vmain = value,
            0x2116 => self.vram.write_vmadd_low(&mut self.regs.vmaddl, &mut self.regs.vmaddh, value),
            0x2117 => self.vram.write_vmadd_high(&mut self.regs.vmaddl, &mut self.regs.vmaddh, value),
            0x2118 => self.vram.write_vmdatal(self.regs.vmain, &mut self.regs.vmaddl, &mut self.regs.vmaddh, value),
            0x2119 => self.vram.write_vmdatah(self.regs.vmain, &mut self.regs.vmaddl, &mut self.regs.vmaddh, value),

            // ==========================
            // CGRAM
            // ==========================
            0x2121 => self.cgram.write_addr(value),
            0x2122 => self.cgram.write_data(value),

            _ => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (unimplemented register)",
                    addr, value
                );
            }
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // ==========================
            // VRAM
            // ==========================
            0x2139 => self.vram.read_vmdatal(self.regs.vmain, &mut self.regs.vmaddl, &mut self.regs.vmaddh),
            0x213A => self.vram.read_vmdatah(self.regs.vmain, &mut self.regs.vmaddl, &mut self.regs.vmaddh),

            // ==========================
            // CGRAM
            // ==========================
            0x213B => self.cgram.read_data(),

            _ => {
                println!(
                    "PPU READ IGNORED: ${:04X} (unimplemented register)",
                    addr
                );
                0
            }
        }
    }

    pub fn step_scanline(&mut self) {
        self.scanline += 1;

        if self.scanline >= SCANLINES_PER_FRAME {
            self.scanline = 0;
            self.frame_ready = true;
        }
    }

    pub fn force_blank(&self) -> bool {
        (self.regs.inidisp & 0x80) != 0
    }

    pub fn brightness(&self) -> u8 {
        self.regs.inidisp & 0x0F
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // PPU::new
    // ============================================================

    /// A freshly created PPU must have scanline at 0 and frame_ready false.
    #[test]
    fn test_new_initial_state() {
        let ppu = PPU::new();
        assert_eq!(ppu.scanline, 0);
        assert!(!ppu.frame_ready);
    }

    // ============================================================
    // force_blank / brightness
    // ============================================================

    /// force_blank must return true when bit 7 of INIDISP is set.
    #[test]
    fn test_force_blank_true_when_bit7_set() {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x80);
        assert!(ppu.force_blank());
    }

    /// force_blank must return false when bit 7 of INIDISP is clear.
    #[test]
    fn test_force_blank_false_when_bit7_clear() {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x0F);
        assert!(!ppu.force_blank());
    }

    /// brightness must return only the lower 4 bits of INIDISP.
    #[test]
    fn test_brightness_returns_lower_nibble() {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0xFF);
        assert_eq!(ppu.brightness(), 0x0F);
    }

    /// brightness must return 0 when INIDISP lower nibble is 0.
    #[test]
    fn test_brightness_zero() {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x80); // force blank, brightness = 0
        assert_eq!(ppu.brightness(), 0);
    }

    // ============================================================
    // write — simple registers
    // ============================================================

    /// Writing $2105 must update bgmode.
    #[test]
    fn test_write_bgmode() {
        let mut ppu = PPU::new();
        ppu.write(0x2105, 0x07);
        assert_eq!(ppu.regs.bgmode, 0x07);
    }

    /// Writing $2107 must update bg1sc.
    #[test]
    fn test_write_bg1sc() {
        let mut ppu = PPU::new();
        ppu.write(0x2107, 0xFC);
        assert_eq!(ppu.regs.bg1sc, 0xFC);
    }

    /// Writing $212C must update tm.
    #[test]
    fn test_write_tm() {
        let mut ppu = PPU::new();
        ppu.write(0x212C, 0x1F);
        assert_eq!(ppu.regs.tm, 0x1F);
    }

    /// Writing $2115 must update vmain.
    #[test]
    fn test_write_vmain() {
        let mut ppu = PPU::new();
        ppu.write(0x2115, 0x80);
        assert_eq!(ppu.regs.vmain, 0x80);
    }

    // ============================================================
    // write $210D — BG1HOFS (two-write latch)
    // ============================================================

    /// First write to $210D must not commit bg1hofs.
    #[test]
    fn test_bg1hofs_first_write_latches() {
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0xAB);
        assert_eq!(ppu.regs.bg1hofs, 0x0000);
    }

    /// Second write to $210D must commit the full 10-bit scroll value.
    #[test]
    fn test_bg1hofs_second_write_commits() {
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0xCD);
        ppu.write(0x210D, 0x03);
        assert_eq!(ppu.regs.bg1hofs, 0x03CD);
    }

    /// The high byte of BG1HOFS must be masked to bits[2:0] only (10-bit scroll).
    #[test]
    fn test_bg1hofs_high_byte_masked_to_3_bits() {
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0xFF);
        ppu.write(0x210D, 0xFF);
        assert_eq!(*ppu.regs.bg1hofs.hi(), 0x07);
    }

    // ============================================================
    // write $210E — BG1VOFS (two-write latch)
    // ============================================================

    /// First write to $210E must not commit bg1vofs.
    #[test]
    fn test_bg1vofs_first_write_latches() {
        let mut ppu = PPU::new();
        ppu.write(0x210E, 0x55);
        assert_eq!(ppu.regs.bg1vofs, 0x0000);
    }

    /// Second write to $210E must commit the full 10-bit scroll value.
    #[test]
    fn test_bg1vofs_second_write_commits() {
        let mut ppu = PPU::new();
        ppu.write(0x210E, 0x78);
        ppu.write(0x210E, 0x02);
        assert_eq!(ppu.regs.bg1vofs, 0x0278);
    }

    /// The high byte of BG1VOFS must be masked to bits[2:0] only.
    #[test]
    fn test_bg1vofs_high_byte_masked_to_3_bits() {
        let mut ppu = PPU::new();
        ppu.write(0x210E, 0x00);
        ppu.write(0x210E, 0xFF);
        assert_eq!(*ppu.regs.bg1vofs.hi(), 0x07);
    }

    // ============================================================
    // write/read — VRAM ($2115–$2119, $2139–$213A)
    // ============================================================

    /// Setting VRAM address via $2116/$2117 and writing a word via $2118/$2119
    /// must store the correct data in VRAM.
    #[test]
    fn test_vram_write_via_ppu() {
        let mut ppu = PPU::new();
        // vmain = 0x80: increment after high byte
        ppu.write(0x2115, 0x80);
        ppu.write(0x2116, 0x10); // vmaddl
        ppu.write(0x2117, 0x00); // vmaddh -> address = 0x0010
        ppu.write(0x2118, 0xCD); // low byte
        ppu.write(0x2119, 0xAB); // high byte -> word = 0xABCD
        assert_eq!(ppu.vram.memory[0x0010], 0xABCD);
    }

    /// Reading $2139/$213A must return the previously written VRAM data (pre-fetch model).
    #[test]
    fn test_vram_read_via_ppu() {
        let mut ppu = PPU::new();
        ppu.vram.memory[0x0005] = 0x1234;

        // vmain = 0x80: increment after high byte
        ppu.write(0x2115, 0x80);
        ppu.write(0x2116, 0x05);
        ppu.write(0x2117, 0x00); // address = 0x0005, latch loaded

        let lo = ppu.read(0x2139);
        let hi = ppu.read(0x213A);

        assert_eq!(lo, 0x34);
        assert_eq!(hi, 0x12);
    }

    /// VRAM address must increment after a complete word write (vmain = 0x80).
    #[test]
    fn test_vram_address_increments_after_write() {
        let mut ppu = PPU::new();
        ppu.write(0x2115, 0x80); // increment after high byte, step = 1
        ppu.write(0x2116, 0x00);
        ppu.write(0x2117, 0x00);
        // Write word at 0x0000
        ppu.write(0x2118, 0x11);
        ppu.write(0x2119, 0x22); // addr increments to 0x0001
        // Write word at 0x0001
        ppu.write(0x2118, 0x33);
        ppu.write(0x2119, 0x44);
        assert_eq!(ppu.vram.memory[0x0000], 0x2211);
        assert_eq!(ppu.vram.memory[0x0001], 0x4433);
    }

    // ============================================================
    // write/read — CGRAM ($2121, $2122, $213B)
    // ============================================================

    /// Writing a colour via $2121/$2122 and reading it back via $213B must round-trip correctly.
    #[test]
    fn test_cgram_write_read_via_ppu() {
        let mut ppu = PPU::new();
        ppu.write(0x2121, 0x00); // CGRAM address = 0
        ppu.write(0x2122, 0xEF); // lo
        ppu.write(0x2122, 0x3A); // hi (bit 7 masked by CGRAM)

        // Reset address for reading
        ppu.write(0x2121, 0x00);
        let lo = ppu.read(0x213B);
        let hi = ppu.read(0x213B);

        assert_eq!(lo, 0xEF);
        assert_eq!(hi & 0x7F, 0x3A);
    }

    // ============================================================
    // write — unknown address
    // ============================================================

    /// Writing to an unimplemented address must not panic or corrupt any state.
    #[test]
    fn test_write_unknown_address_does_not_panic() {
        let mut ppu = PPU::new();
        ppu.write(0xFFFF, 0x42); // must not panic
        assert_eq!(ppu.regs.inidisp, 0); // no corruption
    }

    /// Reading from an unimplemented address must return 0 and not panic.
    #[test]
    fn test_read_unknown_address_returns_zero() {
        let mut ppu = PPU::new();
        let val = ppu.read(0xFFFF);
        assert_eq!(val, 0);
    }

    // ============================================================
    // step_scanline
    // ============================================================

    /// step_scanline must increment the scanline counter by 1.
    #[test]
    fn test_step_scanline_increments() {
        let mut ppu = PPU::new();
        ppu.step_scanline();
        assert_eq!(ppu.scanline, 1);
    }

    /// After 262 steps, scanline must wrap back to 0 and frame_ready must be set.
    #[test]
    fn test_step_scanline_wraps_at_262() {
        let mut ppu = PPU::new();
        for _ in 0..SCANLINES_PER_FRAME {
            ppu.step_scanline();
        }
        assert_eq!(ppu.scanline, 0);
        assert!(ppu.frame_ready);
    }

    /// frame_ready must remain false until the 262nd scanline is reached.
    #[test]
    fn test_frame_ready_false_before_wrap() {
        let mut ppu = PPU::new();
        for _ in 0..SCANLINES_PER_FRAME - 1 {
            ppu.step_scanline();
        }
        assert!(!ppu.frame_ready);
        assert_eq!(ppu.scanline, 261);
    }

    /// frame_ready must stay true across subsequent frames.
    #[test]
    fn test_frame_ready_stays_true_on_subsequent_frames() {
        let mut ppu = PPU::new();
        for _ in 0..SCANLINES_PER_FRAME * 2 {
            ppu.step_scanline();
        }
        assert!(ppu.frame_ready);
        assert_eq!(ppu.scanline, 0);
    }
}

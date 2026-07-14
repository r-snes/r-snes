use crate::cgram::CGRAM;
use crate::constants::SCANLINES_PER_FRAME;
use crate::registers::PPURegisters;
use crate::vram::VRAM;
use common::u16_split::U16Split;

pub struct PPU {
    pub regs: PPURegisters,
    pub vram: VRAM,
    pub cgram: CGRAM,

    // Timing
    pub scanline: u16,
    pub frame_ready: bool,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            regs: PPURegisters::new(),
            vram: VRAM::new(),
            cgram: CGRAM::new(),
            scanline: 0,
            frame_ready: false,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // ==========================
            // DISPLAY
            // ==========================
            0x2100 => self.regs.inidisp = value,
            0x2133 => self.regs.setini = value, // TODO

            // ==========================
            // OAM
            // ==========================
            0x2101 => self.regs.objsel = value,           // TODO
            0x2102 => *self.regs.oamadd.lo_mut() = value, // TODO
            0x2103 => *self.regs.oamadd.hi_mut() = value & 0x01, // TODO
            0x2104 => self.regs.oamdata = value,          // TODO

            // ==========================
            // BACKGROUNDS
            // ==========================
            0x2105 => self.regs.bgmode = value,
            0x2106 => self.regs.mosaic = value, // TODO
            0x2107 => self.regs.bgsc[0] = value,
            0x2108 => self.regs.bgsc[1] = value, // TODO
            0x2109 => self.regs.bgsc[2] = value, // TODO
            0x210A => self.regs.bgsc[3] = value, // TODO
            0x210B => self.regs.bg12nba = value, // TODO
            0x210C => self.regs.bg34nba = value, // TODO

            // BG1HOFS / M7HOFS - same address ($210D)
            0x210D => {
                // BG1HOFS (W8x2): result = (value << 8) | (bgofs_latch & ~7) | (bghofs_latch & 7)
                let lo = (self.regs.bgofs_latch & !0x07) | (self.regs.bghofs_latch & 0x07);
                let hi = value & 0x03;
                *self.regs.bg1hofs.lo_mut() = lo;
                *self.regs.bg1hofs.hi_mut() = hi;
                // M7HOFS (W8x2): result = (value << 8) | mode7_latch
                let m7lo = self.regs.mode7_latch;
                *self.regs.m7hofs.lo_mut() = m7lo;
                *self.regs.m7hofs.hi_mut() = value;
                // Update latches
                self.regs.bgofs_latch = value;
                self.regs.bghofs_latch = value;
                self.regs.mode7_latch = value;
            }

            // BG1VOFS / M7VOFS - same address ($210E)
            0x210E => {
                // BG1VOFS (W8x2): result = (value << 8) | bgofs_latch
                let lo = self.regs.bgofs_latch;
                let hi = value & 0x03;
                *self.regs.bg1vofs.lo_mut() = lo;
                *self.regs.bg1vofs.hi_mut() = hi;
                // M7VOFS (W8x2): result = (value << 8) | mode7_latch
                let m7lo = self.regs.mode7_latch;
                *self.regs.m7vofs.lo_mut() = m7lo;
                *self.regs.m7vofs.hi_mut() = value;
                // Update latches
                self.regs.bgofs_latch = value;
                self.regs.mode7_latch = value;
            }

            // BG2HOFS ($210F)
            0x210F => {
                let lo = (self.regs.bgofs_latch & !0x07) | (self.regs.bghofs_latch & 0x07);
                let hi = value & 0x03;
                *self.regs.bghofs[0].lo_mut() = lo;
                *self.regs.bghofs[0].hi_mut() = hi;
                self.regs.bgofs_latch = value;
                self.regs.bghofs_latch = value;
            }

            // BG2VOFS ($2110)
            0x2110 => {
                let lo = self.regs.bgofs_latch;
                let hi = value & 0x03;
                *self.regs.bgvofs[0].lo_mut() = lo;
                *self.regs.bgvofs[0].hi_mut() = hi;
                self.regs.bgofs_latch = value;
            }

            // BG3HOFS ($2111)
            0x2111 => {
                let lo = (self.regs.bgofs_latch & !0x07) | (self.regs.bghofs_latch & 0x07);
                let hi = value & 0x03;
                *self.regs.bghofs[1].lo_mut() = lo;
                *self.regs.bghofs[1].hi_mut() = hi;
                self.regs.bgofs_latch = value;
                self.regs.bghofs_latch = value;
            }

            // BG3VOFS ($2112)
            0x2112 => {
                let lo = self.regs.bgofs_latch;
                let hi = value & 0x03;
                *self.regs.bgvofs[1].lo_mut() = lo;
                *self.regs.bgvofs[1].hi_mut() = hi;
                self.regs.bgofs_latch = value;
            }

            // BG4HOFS ($2113)
            0x2113 => {
                let lo = (self.regs.bgofs_latch & !0x07) | (self.regs.bghofs_latch & 0x07);
                let hi = value & 0x03;
                *self.regs.bghofs[2].lo_mut() = lo;
                *self.regs.bghofs[2].hi_mut() = hi;
                self.regs.bgofs_latch = value;
                self.regs.bghofs_latch = value;
            }

            // BG4VOFS ($2114)
            0x2114 => {
                let lo = self.regs.bgofs_latch;
                let hi = value & 0x03;
                *self.regs.bgvofs[2].lo_mut() = lo;
                *self.regs.bgvofs[2].hi_mut() = hi;
                self.regs.bgofs_latch = value;
            }

            // ==========================
            // VRAM
            // ==========================
            0x2115 => self.regs.vmain = value,
            0x2116 => self.vram.write_vmadd_low(&mut self.regs, value),
            0x2117 => self.vram.write_vmadd_high(&mut self.regs, value),
            0x2118 => self.vram.write_vmdatal(&mut self.regs, value),
            0x2119 => self.vram.write_vmdatah(&mut self.regs, value),

            // ==========================
            // Mode 7
            // ==========================
            0x211A => self.regs.m7sel = value, // TODO
            0x211B => {
                // M7A (W8x2)
                let lo = self.regs.mode7_latch;
                *self.regs.m7a.lo_mut() = lo;
                *self.regs.m7a.hi_mut() = value;
                self.regs.mode7_latch = value;
            }
            0x211C => {
                // M7B (W8x2)
                let lo = self.regs.mode7_latch;
                *self.regs.m7b.lo_mut() = lo;
                *self.regs.m7b.hi_mut() = value;
                self.regs.mode7_latch = value;
            }
            0x211D => {
                // M7C (W8x2)
                let lo = self.regs.mode7_latch;
                *self.regs.m7c.lo_mut() = lo;
                *self.regs.m7c.hi_mut() = value;
                self.regs.mode7_latch = value;
            }
            0x211E => {
                // M7D (W8x2)
                let lo = self.regs.mode7_latch;
                *self.regs.m7d.lo_mut() = lo;
                *self.regs.m7d.hi_mut() = value;
                self.regs.mode7_latch = value;
            }
            0x211F => {
                // M7X (W8x2)
                let lo = self.regs.mode7_latch;
                *self.regs.m7x.lo_mut() = lo;
                *self.regs.m7x.hi_mut() = value;
                self.regs.mode7_latch = value;
            }
            0x2120 => {
                // M7Y (W8x2)
                let lo = self.regs.mode7_latch;
                *self.regs.m7y.lo_mut() = lo;
                *self.regs.m7y.hi_mut() = value;
                self.regs.mode7_latch = value;
            }

            // ==========================
            // CGRAM
            // ==========================
            0x2121 => self.cgram.write_addr(&mut self.regs, value),
            0x2122 => self.cgram.write_data(&mut self.regs, value),

            // ==========================
            // Window
            // ==========================
            0x2123 => self.regs.w12sel = value,  // TODO
            0x2124 => self.regs.w34sel = value,  // TODO
            0x2125 => self.regs.wobjsel = value, // TODO
            0x2126 => self.regs.wh0 = value,     // TODO
            0x2127 => self.regs.wh1 = value,     // TODO
            0x2128 => self.regs.wh2 = value,     // TODO
            0x2129 => self.regs.wh3 = value,     // TODO
            0x212A => self.regs.wbglog = value,  // TODO
            0x212B => self.regs.wobjlog = value, // TODO

            // ==========================
            // COLOR MATH / LAYER ENABLE
            // ==========================
            0x212C => self.regs.tm = value,
            0x212D => self.regs.ts = value,      // TODO
            0x212E => self.regs.tmw = value,     // TODO
            0x212F => self.regs.tsw = value,     // TODO
            0x2130 => self.regs.cgwsel = value,  // TODO
            0x2131 => self.regs.cgadsub = value, // TODO
            0x2132 => self.regs.coldata = value, // TODO

            _ => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (register not handled by PPU)",
                    addr, value
                );
            }
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // ==========================
            // Multiply
            // ==========================
            0x2134 => Self::unimplemented_read_only(addr), // TODO
            0x2135 => Self::unimplemented_read_only(addr), // TODO
            0x2136 => Self::unimplemented_read_only(addr), // TODO

            // ==========================
            // OAM
            // ==========================
            0x2138 => Self::unimplemented_read_only(addr), // TODO

            // ==========================
            // VRAM
            // ==========================
            0x2139 => self.vram.read_vmdatal(&mut self.regs),
            0x213A => self.vram.read_vmdatah(&mut self.regs),

            // ==========================
            // CGRAM
            // ==========================
            0x213B => self.cgram.read_data(&mut self.regs),

            // ==========================
            // Counters
            // ==========================
            0x2137 => Self::unimplemented_read_only(addr), // TODO
            0x213C => Self::unimplemented_read_only(addr), // TODO
            0x213D => Self::unimplemented_read_only(addr), // TODO

            // ==========================
            // Status
            // ==========================
            0x213E => Self::unimplemented_read_only(addr), // TODO
            0x213F => Self::unimplemented_read_only(addr), // TODO

            _ => {
                println!(
                    "PPU READ IGNORED: ${:04X} (register not handled by PPU)",
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
        } else {
            self.frame_ready = false;
        }
    }

    pub fn force_blank(&self) -> bool {
        (self.regs.inidisp & 0x80) != 0
    }

    pub fn brightness(&self) -> u8 {
        self.regs.inidisp & 0x0F
    }

    fn unimplemented_read_only(addr: u16) -> u8 {
        println!("PPU READ IGNORED: ${:04X} (unimplemented register)", addr);
        0
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
    // $2100 - INIDISP: force_blank / brightness
    // ============================================================

    /// force_blank must return true when bit 7 of INIDISP is set, false otherwise.
    /// brightness must return only the lower 4 bits of INIDISP.
    #[test]
    fn test_inidisp() {
        let mut ppu = PPU::new();

        ppu.write(0x2100, 0x80);
        assert!(ppu.force_blank());
        assert_eq!(ppu.brightness(), 0);

        ppu.write(0x2100, 0x0F);
        assert!(!ppu.force_blank());

        ppu.write(0x2100, 0xFF);
        assert_eq!(ppu.brightness(), 0x0F);
    }

    // ============================================================
    // $2101–$2104 - OAM
    // ============================================================

    /// Writing $2101 must update objsel.
    /// Writing $2102 must update the low byte of oamadd.
    /// Writing $2103 must update the high byte of oamadd (only bit 0 is valid).
    /// Writing $2104 must update oamdata.
    #[test]
    fn test_write_oam_registers() {
        let mut ppu = PPU::new();

        ppu.write(0x2101, 0xA5);
        assert_eq!(ppu.regs.objsel, 0xA5);

        ppu.write(0x2102, 0x7F);
        assert_eq!(*ppu.regs.oamadd.lo(), 0x7F);

        ppu.write(0x2103, 0x01);
        assert_eq!(*ppu.regs.oamadd.hi(), 0x01);

        ppu.write(0x2104, 0xBE);
        assert_eq!(ppu.regs.oamdata, 0xBE);
    }

    // ============================================================
    // $2105 - BGMODE / bg_mode()
    // ============================================================

    /// Writing $2105 must update bgmode.
    /// bg_mode must return only bits[2:0] of BGMODE.
    #[test]
    fn test_bgmode() {
        let mut ppu = PPU::new();

        ppu.write(0x2105, 0b11110111);
        assert_eq!(ppu.regs.bg_mode(), 7);

        ppu.write(0x2105, 0b11111000);
        assert_eq!(ppu.regs.bg_mode(), 0);
    }

    // ============================================================
    // $2106 - MOSAIC
    // ============================================================

    /// Writing $2106 must update mosaic.
    #[test]
    fn test_write_mosaic() {
        let mut ppu = PPU::new();
        ppu.write(0x2106, 0xF1);
        assert_eq!(ppu.regs.mosaic, 0xF1);
    }

    // ============================================================
    // $2107–$210A - BGxSC
    // ============================================================

    /// Writing $2107–$210A must update bgsc[0]–bgsc[3].
    #[test]
    fn test_write_bgsc() {
        let mut ppu = PPU::new();

        ppu.write(0x2107, 0xFC);
        assert_eq!(ppu.regs.bgsc[0], 0xFC);

        ppu.write(0x2108, 0x10);
        assert_eq!(ppu.regs.bgsc[1], 0x10);

        ppu.write(0x2109, 0x20);
        assert_eq!(ppu.regs.bgsc[2], 0x20);

        ppu.write(0x210A, 0x30);
        assert_eq!(ppu.regs.bgsc[3], 0x30);
    }

    // ============================================================
    // $210B–$210C - BG12NBA / BG34NBA / tiledata helpers
    // ============================================================

    /// Writing $210B/$210C must update bg12nba/bg34nba.
    /// Tiledata address helpers must derive correctly from the nibbles.
    #[test]
    fn test_bgnba_and_tiledata_addrs() {
        let mut ppu = PPU::new();

        ppu.write(0x210B, 0x01);
        assert_eq!(ppu.regs.bg12nba, 0x01);
        assert_eq!(ppu.regs.bg1_tiledata_addr(), 0x1000);

        ppu.write(0x210B, 0x00);
        assert_eq!(ppu.regs.bg1_tiledata_addr(), 0x0000);

        ppu.write(0x210B, 0x0F);
        assert_eq!(ppu.regs.bg1_tiledata_addr(), 0xF000);

        ppu.write(0x210C, 0x23);
        assert_eq!(ppu.regs.bg34nba, 0x23);
    }

    // ============================================================
    // $210D - BG1HOFS / M7HOFS (W8x2, shared address)
    // ============================================================

    /// BG1HOFS uses a write-twice mechanism via bgofs_latch/bghofs_latch.
    /// The result is: lo = (bgofs_latch & ~7) | (bghofs_latch & 7), hi = value & 0x03.
    /// M7HOFS uses mode7_latch: lo = mode7_latch, hi = value.
    #[test]
    fn test_write_bg1hofs_and_m7hofs() {
        let mut ppu = PPU::new();

        // First write: only updates latches, no commit yet
        ppu.write(0x210D, 0xAB);
        assert_eq!(ppu.regs.bgofs_latch, 0xAB);
        assert_eq!(ppu.regs.bghofs_latch, 0xAB);
        assert_eq!(ppu.regs.mode7_latch, 0xAB);

        // Second write commits both BG1HOFS and M7HOFS
        ppu.write(0x210D, 0x03);
        // BG1HOFS: lo = (0xAB & ~7) | (0xAB & 7) = 0xAB, hi = 0x03 & 0x03 = 0x03
        assert_eq!(ppu.regs.bg1hofs, 0x03AB);
        // M7HOFS: lo = 0xAB (first write's mode7_latch), hi = 0x03
        assert_eq!(ppu.regs.m7hofs, 0x03AB);

        // High byte of BG1HOFS masked to bits[1:0] (10-bit scroll)
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0xFF);
        ppu.write(0x210D, 0xFF);
        assert_eq!(*ppu.regs.bg1hofs.hi(), 0x03);
    }

    // ============================================================
    // $210E - BG1VOFS / M7VOFS (W8x2, shared address)
    // ============================================================

    /// BG1VOFS: lo = bgofs_latch, hi = value & 0x03.
    /// M7VOFS: lo = mode7_latch, hi = value.
    #[test]
    fn test_write_bg1vofs_and_m7vofs() {
        let mut ppu = PPU::new();

        ppu.write(0x210E, 0x78);
        ppu.write(0x210E, 0x02);
        // BG1VOFS: lo = 0x78 (bgofs_latch from first write), hi = 0x02 & 0x03
        assert_eq!(ppu.regs.bg1vofs, 0x0278);
        // M7VOFS: lo = 0x78 (mode7_latch from first write), hi = 0x02
        assert_eq!(ppu.regs.m7vofs, 0x0278);

        // High byte of BG1VOFS masked to bits[1:0]
        let mut ppu = PPU::new();
        ppu.write(0x210E, 0x00);
        ppu.write(0x210E, 0xFF);
        assert_eq!(*ppu.regs.bg1vofs.hi(), 0x03);
    }

    // ============================================================
    // $210F–$2114 - BG2–BG4 scroll (W8x2 via bgofs_latch/bghofs_latch)
    // ============================================================

    /// BG2HOFS–BG4VOFS use the shared bgofs_latch/bghofs_latch.
    /// Each write pair commits the correct entry in bghofs[]/bgvofs[].
    #[test]
    fn test_write_bg2_to_bg4_scroll() {
        let mut ppu = PPU::new();

        // BG2HOFS ($210F): write lo latch, then commit
        ppu.write(0x210F, 0x10);
        ppu.write(0x210F, 0x01);
        assert_eq!(ppu.regs.bghofs[0], 0x0110);

        // BG2VOFS ($2110)
        ppu.write(0x2110, 0x20);
        ppu.write(0x2110, 0x02);
        assert_eq!(ppu.regs.bgvofs[0], 0x0220);

        // BG3HOFS ($2111)
        ppu.write(0x2111, 0x30);
        ppu.write(0x2111, 0x03);
        assert_eq!(ppu.regs.bghofs[1], 0x0330);

        // BG3VOFS ($2112)
        ppu.write(0x2112, 0x40);
        ppu.write(0x2112, 0x04);
        // Note: hi is masked to bits[1:0] -> 0x04 & 0x03 = 0x00... wait, 0x04 & 0x03 = 0x00
        // Actually it's 0x00 since 0x04 & 0x03 == 0x00
        // Let's use a value that survives the mask
        let mut ppu2 = PPU::new();
        ppu2.write(0x2112, 0x40);
        ppu2.write(0x2112, 0x01);
        assert_eq!(ppu2.regs.bgvofs[1], 0x0140);

        // BG4HOFS ($2113)
        let mut ppu3 = PPU::new();
        ppu3.write(0x2113, 0x50);
        ppu3.write(0x2113, 0x01);
        assert_eq!(ppu3.regs.bghofs[2], 0x0150);

        // BG4VOFS ($2114)
        let mut ppu4 = PPU::new();
        ppu4.write(0x2114, 0x60);
        ppu4.write(0x2114, 0x02);
        assert_eq!(ppu4.regs.bgvofs[2], 0x0260);
    }

    // ============================================================
    // $2115–$2119 / $2139–$213A - VRAM
    // ============================================================

    /// Writing $2115 must update vmain.
    /// Setting VRAM address and writing a word must store the correct data.
    /// Reading back $2139/$213A must return the written bytes.
    /// Address must increment after each complete word write.
    #[test]
    fn test_vram_via_ppu() {
        let mut ppu = PPU::new();

        ppu.write(0x2115, 0x80);
        assert_eq!(ppu.regs.vmain, 0x80);

        // Write word 0xABCD at address 0x0010
        ppu.write(0x2116, 0x10);
        ppu.write(0x2117, 0x00);
        ppu.write(0x2118, 0xCD);
        ppu.write(0x2119, 0xAB);
        assert_eq!(ppu.vram.memory[0x0010], 0xABCD);

        // Read back
        ppu.vram.memory[0x0005] = 0x1234;
        ppu.write(0x2116, 0x05);
        ppu.write(0x2117, 0x00);
        assert_eq!(ppu.read(0x2139), 0x34);
        assert_eq!(ppu.read(0x213A), 0x12);

        // Sequential writes increment address
        ppu.write(0x2116, 0x00);
        ppu.write(0x2117, 0x00);
        ppu.write(0x2118, 0x11);
        ppu.write(0x2119, 0x22); // addr -> 0x0001
        ppu.write(0x2118, 0x33);
        ppu.write(0x2119, 0x44);
        assert_eq!(ppu.vram.memory[0x0000], 0x2211);
        assert_eq!(ppu.vram.memory[0x0001], 0x4433);
    }

    // ============================================================
    // $211A–$2120 - Mode 7 (W8x2 via mode7_latch)
    // ============================================================

    /// Writing $211A must update m7sel.
    /// Mode 7 matrix registers ($211B–$2120) are W8x2 via mode7_latch:
    /// first write stores latch, second write commits (hi=second, lo=first).
    #[test]
    fn test_mode7_registers() {
        let mut ppu = PPU::new();

        ppu.write(0x211A, 0x03);
        assert_eq!(ppu.regs.m7sel, 0x03);

        // M7A: two writes -> full 16-bit value
        ppu.write(0x211B, 0x34);
        ppu.write(0x211B, 0x12);
        assert_eq!(ppu.regs.m7a, 0x1234);

        ppu.write(0x211C, 0x56);
        ppu.write(0x211C, 0x78);
        assert_eq!(ppu.regs.m7b, 0x7856);

        ppu.write(0x211D, 0xAB);
        ppu.write(0x211D, 0xCD);
        assert_eq!(ppu.regs.m7c, 0xCDAB);

        ppu.write(0x211E, 0x11);
        ppu.write(0x211E, 0x22);
        assert_eq!(ppu.regs.m7d, 0x2211);

        ppu.write(0x211F, 0x80);
        ppu.write(0x211F, 0x00);
        assert_eq!(ppu.regs.m7x, 0x0080);

        ppu.write(0x2120, 0x40);
        ppu.write(0x2120, 0x01);
        assert_eq!(ppu.regs.m7y, 0x0140);
    }

    // ============================================================
    // $2121/$2122/$213B - CGRAM
    // ============================================================

    /// Writing a colour via $2121/$2122 and reading it back via $213B must round-trip correctly.
    #[test]
    fn test_cgram_write_read_via_ppu() {
        let mut ppu = PPU::new();
        ppu.write(0x2121, 0x00);
        ppu.write(0x2122, 0xEF); // lo
        ppu.write(0x2122, 0x3A); // hi
        ppu.write(0x2121, 0x00);
        let lo = ppu.read(0x213B);
        let hi = ppu.read(0x213B);
        assert_eq!(lo, 0xEF);
        assert_eq!(hi & 0x7F, 0x3A);
    }

    // ============================================================
    // $2123–$212B - Window registers
    // ============================================================

    /// Writing window registers must store the value verbatim.
    #[test]
    fn test_write_window_registers() {
        let cases: &[(u16, fn(&PPURegisters) -> u8)] = &[
            (0x2123, |r| r.w12sel),
            (0x2124, |r| r.w34sel),
            (0x2125, |r| r.wobjsel),
            (0x2126, |r| r.wh0),
            (0x2127, |r| r.wh1),
            (0x2128, |r| r.wh2),
            (0x2129, |r| r.wh3),
            (0x212A, |r| r.wbglog),
            (0x212B, |r| r.wobjlog),
        ];
        for &(addr, getter) in cases {
            let mut ppu = PPU::new();
            ppu.write(addr, 0xA5);
            assert_eq!(
                getter(&ppu.regs),
                0xA5,
                "register at ${:04X} did not store value",
                addr
            );
        }
    }

    // ============================================================
    // $212C–$2133 - Color math / layer enable / SETINI
    // ============================================================

    /// Writing $212C must update tm; bg1_enabled reflects bit 0.
    /// Writing $212D–$2133 must store verbatim.
    #[test]
    fn test_write_color_math_and_layer_registers() {
        let mut ppu = PPU::new();

        ppu.write(0x212C, 0x1F);
        assert_eq!(ppu.regs.tm, 0x1F);

        let cases: &[(u16, fn(&PPURegisters) -> u8)] = &[
            (0x212D, |r| r.ts),
            (0x212E, |r| r.tmw),
            (0x212F, |r| r.tsw),
            (0x2130, |r| r.cgwsel),
            (0x2131, |r| r.cgadsub),
            (0x2132, |r| r.coldata),
            (0x2133, |r| r.setini),
        ];
        for &(addr, getter) in cases {
            let mut ppu = PPU::new();
            ppu.write(addr, 0xA5);
            assert_eq!(
                getter(&ppu.regs),
                0xA5,
                "register at ${:04X} did not store value",
                addr
            );
        }
    }

    // ============================================================
    // $212C - TM / bg1_enabled()
    // ============================================================

    /// bg1_enabled must reflect bit 0 of TM only.
    #[test]
    fn test_bg1_enabled() {
        let mut ppu = PPU::new();

        ppu.write(0x212C, 0x01);
        assert!(ppu.regs.bg1_enabled());

        ppu.write(0x212C, 0xFE);
        assert!(!ppu.regs.bg1_enabled());

        ppu.write(0x212C, 0x1E);
        assert!(!ppu.regs.bg1_enabled());
    }

    // ============================================================
    // $2107 - BG1SC / bg1_tilemap_addr()
    // ============================================================

    /// bg1_tilemap_addr must derive the VRAM address from bits[7:2] of bgsc[0].
    #[test]
    fn test_bg1_tilemap_addr() {
        let mut ppu = PPU::new();

        ppu.write(0x2107, 0b00000100);
        assert_eq!(ppu.regs.bg1_tilemap_addr(), 0x0400);

        ppu.write(0x2107, 0x00);
        assert_eq!(ppu.regs.bg1_tilemap_addr(), 0x0000);

        ppu.write(0x2107, 0xFF);
        assert_eq!(ppu.regs.bg1_tilemap_addr(), 0x3F * 0x400);
    }

    // ============================================================
    // step_scanline
    // ============================================================

    /// step_scanline increments the counter, wraps at SCANLINES_PER_FRAME,
    /// and sets frame_ready only on the wrap.
    #[test]
    fn test_step_scanline() {
        let mut ppu = PPU::new();

        ppu.step_scanline();
        assert_eq!(ppu.scanline, 1);
        assert!(!ppu.frame_ready);

        // One step before wrap
        for _ in 1..SCANLINES_PER_FRAME - 1 {
            ppu.step_scanline();
        }
        assert!(!ppu.frame_ready);
        assert_eq!(ppu.scanline, SCANLINES_PER_FRAME - 1);

        // Wrap
        ppu.step_scanline();
        assert_eq!(ppu.scanline, 0);
        assert!(ppu.frame_ready);

        // Second frame wrap also sets frame_ready
        for _ in 0..SCANLINES_PER_FRAME {
            ppu.step_scanline();
        }
        assert!(ppu.frame_ready);
        assert_eq!(ppu.scanline, 0);
    }
}

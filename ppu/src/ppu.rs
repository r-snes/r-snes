use crate::cgram::CGRAM;
use crate::constants::*;
use crate::registers::PPURegisters;
use crate::vram::VRAM;
use common::u16_split::U16Split;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PpuEvent {
    /// Nothing crossed.
    None,
    /// A new dot began mid-scanline.
    DotStart,
    /// A new dot began, and it is the first dot of H-Blank.
    HBlankStart,
    /// A new scanline began. Implies a new dot and the end of H-Blank.
    ScanlineStart(ScanlineKind),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScanlineKind {
    /// Any line that isn't a V-Blank boundary, visible or not.
    Normal,
    /// First line of V-Blank (225, or 240 with overscan).
    VBlankStart,
    /// Line 0: V-Blank ends and a new frame begins.
    FrameStart,
}

pub struct PPU {
    pub regs: PPURegisters,
    pub vram: VRAM,
    pub cgram: CGRAM,

    // Timing
    pub scanline: u16,
    /// Master cycles elapsed inside the current scanline (0..1364).
    pub h_cycles: u32,
    pub frame: u64,
    pub odd_frame: bool,
}

impl Default for PPU {
    fn default() -> Self {
        Self::new()
    }
}

impl PPU {
    pub fn new() -> Self {
        Self {
            regs: PPURegisters::new(),
            vram: VRAM::new(),
            cgram: CGRAM::new(),
            scanline: 0,
            h_cycles: 0,
            frame: 0,
            odd_frame: false,
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

    /// Current dot (0..339).
    pub fn dot(&self) -> u16 {
        (self.h_cycles / 4) as u16
    }

    /// Non-interlace odd frames shorten scanline 240 to 1360 cycles.
    fn scanline_length(&self) -> u32 {
        if self.odd_frame && self.scanline == 240 {
            MASTER_CYCLES_PER_SCANLINE - 4
        } else {
            MASTER_CYCLES_PER_SCANLINE
        }
    }

    pub fn vblank_start_line(&self) -> u16 {
        if self.regs.setini & 0x04 != 0 {
            VBLANK_START_LINE_OVERSCAN
        } else {
            VBLANK_START_LINE
        }
    }

    /// Framebuffer row this scanline draws into, if it's a visible one.
    /// Scanline 0 is the dummy/pre-render line; 1..=224 are visible.
    pub fn visible_line(&self) -> Option<usize> {
        (1..=SCREEN_HEIGHT as u16)
            .contains(&self.scanline)
            .then(|| self.scanline as usize - 1)
    }

    /// Advance one master cycle.
    pub fn tick(&mut self) -> PpuEvent {
        let prev_dot = self.dot();
        self.h_cycles += 1;

        if self.h_cycles >= self.scanline_length() {
            self.h_cycles = 0;
            self.scanline += 1;

            let kind = if self.scanline >= SCANLINES_PER_FRAME {
                self.scanline = 0;
                self.frame += 1;
                self.odd_frame = !self.odd_frame;
                ScanlineKind::FrameStart
            } else if self.scanline == self.vblank_start_line() {
                ScanlineKind::VBlankStart
            } else {
                ScanlineKind::Normal
            };

            PpuEvent::ScanlineStart(kind)
        } else if self.dot() != prev_dot {
            if self.dot() == HBLANK_START_DOT {
                PpuEvent::HBlankStart
            } else {
                PpuEvent::DotStart
            }
        } else {
            PpuEvent::None
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

    // (register address, getter into PPURegisters)
    type RegCase = (u16, fn(&PPURegisters) -> u8);

    // ============================================================
    // PPU::new
    // ============================================================

    /// A freshly created PPU sits at the very start of scanline 0 of frame 0.
    #[test]
    fn test_new_initial_state() {
        let ppu = PPU::new();
        assert_eq!(ppu.scanline, 0);
        assert_eq!(ppu.h_cycles, 0);
        assert_eq!(ppu.dot(), 0);
        assert_eq!(ppu.frame, 0);
        assert!(!ppu.odd_frame);
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
        let cases: &[RegCase] = &[
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

        let cases: &[RegCase] = &[
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
    // Timing helpers
    // ============================================================

    /// Ticks until the PPU sits at the very start of `target`, and returns
    /// the event raised on arrival. Panics rather than spinning forever if
    /// the scanline is never reached.
    ///
    /// Always ticks at least once, so calling it with the PPU already at
    /// `target` advances a full frame.
    fn advance_to_scanline_start(ppu: &mut PPU, target: u16) -> PpuEvent {
        let cap = (SCANLINES_PER_FRAME as u32 + 1) * MASTER_CYCLES_PER_SCANLINE;
        for _ in 0..cap {
            let ev = ppu.tick();
            if ppu.scanline == target && ppu.h_cycles == 0 {
                return ev;
            }
        }
        panic!("never reached the start of scanline {target}");
    }

    /// Ticks through one whole frame and returns how many master cycles it took.
    fn count_frame_cycles(ppu: &mut PPU) -> u32 {
        let mut cycles = 0;
        loop {
            cycles += 1;
            if ppu.tick() == PpuEvent::ScanlineStart(ScanlineKind::FrameStart) {
                return cycles;
            }
        }
    }

    // ============================================================
    // tick() - dot progression
    // ============================================================

    /// A dot is 4 master cycles, so only every fourth tick advances it.
    #[test]
    fn test_tick_dot_progression() {
        let mut ppu = PPU::new();

        for _ in 0..3 {
            assert_eq!(ppu.tick(), PpuEvent::None);
            assert_eq!(ppu.dot(), 0);
        }

        assert_eq!(ppu.tick(), PpuEvent::DotStart);
        assert_eq!(ppu.dot(), 1);
        assert_eq!(ppu.h_cycles, 4);
    }

    /// H-Blank is announced exactly once, on the first cycle of dot 274.
    #[test]
    fn test_tick_hblank_start() {
        let mut ppu = PPU::new();

        // Dot N begins at master cycle 4*N.
        for _ in 0..(HBLANK_START_DOT as u32 * 4 - 1) {
            assert_ne!(ppu.tick(), PpuEvent::HBlankStart);
        }

        assert_eq!(ppu.tick(), PpuEvent::HBlankStart);
        assert_eq!(ppu.dot(), HBLANK_START_DOT);
    }

    /// Over one full scanline: one ScanlineStart, one HBlankStart, and a
    /// DotStart on every other dot boundary.
    #[test]
    fn test_event_counts_over_one_scanline() {
        let mut ppu = PPU::new();
        let (mut none, mut dots, mut hblanks, mut scanlines) = (0, 0, 0, 0);

        for _ in 0..MASTER_CYCLES_PER_SCANLINE {
            match ppu.tick() {
                PpuEvent::None => none += 1,
                PpuEvent::DotStart => dots += 1,
                PpuEvent::HBlankStart => hblanks += 1,
                PpuEvent::ScanlineStart(_) => scanlines += 1,
            }
        }

        assert_eq!(scanlines, 1);
        assert_eq!(hblanks, 1);
        assert_eq!(dots, 339);
        assert_eq!(none, MASTER_CYCLES_PER_SCANLINE - 341);
    }

    // ============================================================
    // tick() - scanline and frame wrap
    // ============================================================

    /// Crossing a scanline boundary resets h_cycles and bumps the counter.
    #[test]
    fn test_tick_scanline_wrap() {
        let mut ppu = PPU::new();

        for _ in 0..MASTER_CYCLES_PER_SCANLINE - 1 {
            ppu.tick();
        }
        assert_eq!(ppu.scanline, 0);

        assert_eq!(ppu.tick(), PpuEvent::ScanlineStart(ScanlineKind::Normal));
        assert_eq!(ppu.scanline, 1);
        assert_eq!(ppu.h_cycles, 0);
        assert_eq!(ppu.dot(), 0);
    }

    /// Scanline 225 raises VBlankStart; line 0 raises FrameStart and flips
    /// the odd/even field.
    #[test]
    fn test_tick_vblank_and_frame_events() {
        let mut ppu = PPU::new();

        assert_eq!(
            advance_to_scanline_start(&mut ppu, VBLANK_START_LINE),
            PpuEvent::ScanlineStart(ScanlineKind::VBlankStart)
        );

        assert_eq!(
            advance_to_scanline_start(&mut ppu, 0),
            PpuEvent::ScanlineStart(ScanlineKind::FrameStart)
        );
        assert_eq!(ppu.scanline, 0);
        assert_eq!(ppu.frame, 1);
        assert!(ppu.odd_frame);

        advance_to_scanline_start(&mut ppu, 0);
        assert_eq!(ppu.frame, 2);
        assert!(!ppu.odd_frame);
    }

    /// Ordinary scanlines raise ScanlineKind::Normal, not a boundary kind.
    #[test]
    fn test_tick_normal_scanline_kind() {
        let mut ppu = PPU::new();
        assert_eq!(
            advance_to_scanline_start(&mut ppu, 100),
            PpuEvent::ScanlineStart(ScanlineKind::Normal)
        );
    }

    /// Non-interlace odd frames shorten scanline 240 by one dot.
    #[test]
    fn test_short_scanline_on_odd_frames() {
        let mut ppu = PPU::new();
        let full = SCANLINES_PER_FRAME as u32 * MASTER_CYCLES_PER_SCANLINE;

        assert_eq!(count_frame_cycles(&mut ppu), full);
        assert!(ppu.odd_frame);

        assert_eq!(count_frame_cycles(&mut ppu), full - 4);
        assert!(!ppu.odd_frame);

        assert_eq!(count_frame_cycles(&mut ppu), full);
    }

    // ============================================================
    // vblank_start_line - $2133 SETINI overscan
    // ============================================================

    /// SETINI bit 2 moves the start of V-Blank from line 225 to line 240.
    #[test]
    fn test_vblank_start_line_overscan() {
        let mut ppu = PPU::new();
        assert_eq!(ppu.vblank_start_line(), VBLANK_START_LINE);

        ppu.write(0x2133, 0x04);
        assert_eq!(ppu.vblank_start_line(), VBLANK_START_LINE_OVERSCAN);

        // Line 225 is now an ordinary line...
        assert_eq!(
            advance_to_scanline_start(&mut ppu, VBLANK_START_LINE),
            PpuEvent::ScanlineStart(ScanlineKind::Normal)
        );
        // ...and V-Blank starts 15 lines later.
        assert_eq!(
            advance_to_scanline_start(&mut ppu, VBLANK_START_LINE_OVERSCAN),
            PpuEvent::ScanlineStart(ScanlineKind::VBlankStart)
        );
    }

    // ============================================================
    // visible_line
    // ============================================================

    /// Scanline 0 is the pre-render line; 1..=224 map to framebuffer rows
    /// 0..=223; everything from V-Blank on maps to nothing.
    #[test]
    fn test_visible_line() {
        let mut ppu = PPU::new();
        assert_eq!(ppu.visible_line(), None);

        for expected_y in 0..SCREEN_HEIGHT {
            advance_to_scanline_start(&mut ppu, expected_y as u16 + 1);
            assert_eq!(ppu.visible_line(), Some(expected_y));
        }

        advance_to_scanline_start(&mut ppu, VBLANK_START_LINE);
        assert_eq!(ppu.visible_line(), None);
    }
}

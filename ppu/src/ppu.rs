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
            0x2101 => self.regs.objsel = value, // TODO
            0x2102 => self.regs.oamaddl = value, // TODO
            0x2103 => self.regs.oamaddh = value, // TODO
            0x2104 => self.regs.oamdata = value, // TODO

            // ==========================
            // BACKGROUNDS
            // ==========================
            0x2105 => self.regs.bgmode = value,
            0x2106 => self.regs.mosaic = value, // TODO
            0x2107 => self.regs.bg1sc = value,
            0x2108 => self.regs.bg2sc = value, // TODO
            0x2109 => self.regs.bg3sc = value, // TODO
            0x210A => self.regs.bg4sc = value, // TODO
            0x210B => self.regs.bg12nba = value, // TODO
            0x210C => self.regs.bg34nba = value, // TODO

            // BG1 HOFS
            0x210D => {
                if let Some((lo, hi)) = self.regs.bg1hofs_latch.write(value) {
                    *self.regs.bg1hofs.lo_mut() = lo;
                    *self.regs.bg1hofs.hi_mut() = hi & 0x07;
                }
            }

            // BG1 VOFS
            0x210E => {
                if let Some((lo, hi)) = self.regs.bg1vofs_latch.write(value) {
                    *self.regs.bg1vofs.lo_mut() = lo;
                    *self.regs.bg1vofs.hi_mut() = hi & 0x07;
                }
            }

            0x210F => self.regs.bg1vofs = value as u16, // TODO
            0x2110 => self.regs.m7vofs = value as u16, // TODO
            0x2111 => self.regs.bg2hofs = value as u16, // TODO
            0x2112 => self.regs.bg2vofs = value as u16, // TODO
            0x2113 => self.regs.bg3hofs = value as u16, // TODO
            0x2114 => self.regs.bg3vofs = value as u16, // TODO

            // ==========================
            // VRAM
            // ==========================
            0x2115 => self.regs.vmain = value,
            0x2116 => self.vram.write_vmadd_low(&mut self.regs.vmaddl, &mut self.regs.vmaddh, value),
            0x2117 => self.vram.write_vmadd_high(&mut self.regs.vmaddl, &mut self.regs.vmaddh, value),
            0x2118 => self.vram.write_vmdatal(self.regs.vmain, &mut self.regs.vmaddl, &mut self.regs.vmaddh, value),
            0x2119 => self.vram.write_vmdatah(self.regs.vmain, &mut self.regs.vmaddl, &mut self.regs.vmaddh, value),

            // ==========================
            // Mode 7
            // ==========================
            0x211A => self.regs.m7sel = value, // TODO
            0x211B => self.regs.m7a = value as u16, // TODO
            0x211C => self.regs.m7b = value as u16, // TODO
            0x211D => self.regs.m7c = value as u16, // TODO
            0x211E => self.regs.m7d = value as u16, // TODO
            0x211F => self.regs.m7x = value as u16, // TODO
            0x2120 => self.regs.m7y = value as u16, // TODO

            // ==========================
            // CGRAM
            // ==========================
            0x2121 => self.cgram.write_addr(value),
            0x2122 => self.cgram.write_data(value),

            // ==========================
            // Window
            // ==========================
            0x2123 => self.regs.w12sel = value, // TODO
            0x2124 => self.regs.w34sel = value, // TODO
            0x2125 => self.regs.wobjsel = value, // TODO
            0x2126 => self.regs.wh0 = value, // TODO
            0x2127 => self.regs.wh1 = value, // TODO
            0x2128 => self.regs.wh2 = value, // TODO
            0x2129 => self.regs.wh3 = value, // TODO
            0x212A => self.regs.wbglog = value, // TODO
            0x212B => self.regs.wobjlog = value, // TODO

            // ==========================
            // COLOR MATH / LAYER ENABLE
            // ==========================
            0x212C => self.regs.tm = value,
            0x212D => self.regs.ts = value, // TODO
            0x212E => self.regs.tmw = value, // TODO
            0x212F => self.regs.tsw = value, // TODO
            0x2130 => self.regs.cgwsel = value, // TODO
            0x2131 => self.regs.cgadsub = value, // TODO
            0x2132 => self.regs.coldata = value, // TODO

            _ => {
                println!("PPU WRITE IGNORED: ${:04X} = {:02X} (register not handled by PPU)", addr, value);
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
            0x2139 => self.vram.read_vmdatal(self.regs.vmain, &mut self.regs.vmaddl, &mut self.regs.vmaddh),
            0x213A => self.vram.read_vmdatah(self.regs.vmain, &mut self.regs.vmaddl, &mut self.regs.vmaddh),

            // ==========================
            // CGRAM
            // ==========================
            0x213B => self.cgram.read_data(),

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
                println!("PPU READ IGNORED: ${:04X} (register not handled by PPU)", addr);
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

    fn unimplemented_read_only(addr: u16) -> u8 {
        println!(
            "PPU READ IGNORED: ${:04X} (unimplemented register)",
            addr
        );
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
        ppu.write(0x2100, 0x80);
        assert_eq!(ppu.brightness(), 0);
    }

    // ============================================================
    // $2101–$2104 - OAM
    // ============================================================

    /// Writing $2101 must update objsel.
    #[test]
    fn test_write_objsel() {
        let mut ppu = PPU::new();
        ppu.write(0x2101, 0xA5);
        assert_eq!(ppu.regs.objsel, 0xA5);
    }

    /// Writing $2102 must update oamaddl.
    #[test]
    fn test_write_oamaddl() {
        let mut ppu = PPU::new();
        ppu.write(0x2102, 0x7F);
        assert_eq!(ppu.regs.oamaddl, 0x7F);
    }

    /// Writing $2103 must update oamaddh.
    #[test]
    fn test_write_oamaddh() {
        let mut ppu = PPU::new();
        ppu.write(0x2103, 0x01);
        assert_eq!(ppu.regs.oamaddh, 0x01);
    }

    /// Writing $2104 must update oamdata.
    #[test]
    fn test_write_oamdata() {
        let mut ppu = PPU::new();
        ppu.write(0x2104, 0xBE);
        assert_eq!(ppu.regs.oamdata, 0xBE);
    }

    // ============================================================
    // $2105 - BGMODE / bg_mode()
    // ============================================================

    /// Writing $2105 must update bgmode.
    #[test]
    fn test_write_bgmode() {
        let mut ppu = PPU::new();
        ppu.write(0x2105, 0x07);
        assert_eq!(ppu.regs.bgmode, 0x07);
    }

    /// bg_mode must return only bits[2:0] of BGMODE.
    #[test]
    fn test_bg_mode_returns_lower_3_bits() {
        let mut ppu = PPU::new();
        ppu.write(0x2105, 0b11110111);
        assert_eq!(ppu.regs.bg_mode(), 7);
    }

    /// bg_mode must mask out bits above bit 2.
    #[test]
    fn test_bg_mode_masks_upper_bits() {
        let mut ppu = PPU::new();
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

    /// Writing $2107 must update bg1sc.
    #[test]
    fn test_write_bg1sc() {
        let mut ppu = PPU::new();
        ppu.write(0x2107, 0xFC);
        assert_eq!(ppu.regs.bg1sc, 0xFC);
    }

    /// Writing $2108 must update bg2sc.
    #[test]
    fn test_write_bg2sc() {
        let mut ppu = PPU::new();
        ppu.write(0x2108, 0x10);
        assert_eq!(ppu.regs.bg2sc, 0x10);
    }

    /// Writing $2109 must update bg3sc.
    #[test]
    fn test_write_bg3sc() {
        let mut ppu = PPU::new();
        ppu.write(0x2109, 0x20);
        assert_eq!(ppu.regs.bg3sc, 0x20);
    }

    /// Writing $210A must update bg4sc.
    #[test]
    fn test_write_bg4sc() {
        let mut ppu = PPU::new();
        ppu.write(0x210A, 0x30);
        assert_eq!(ppu.regs.bg4sc, 0x30);
    }

    // ============================================================
    // $210B–$210C - BG12NBA / BG34NBA / bg1_tiledata_addr()
    // ============================================================

    /// Writing $210B must update bg12nba.
    #[test]
    fn test_write_bg12nba() {
        let mut ppu = PPU::new();
        ppu.write(0x210B, 0x01);
        assert_eq!(ppu.regs.bg12nba, 0x01);
    }

    /// Writing $210C must update bg34nba.
    #[test]
    fn test_write_bg34nba() {
        let mut ppu = PPU::new();
        ppu.write(0x210C, 0x23);
        assert_eq!(ppu.regs.bg34nba, 0x23);
    }

    /// bg1_tiledata_addr must derive the CHR base address from bits[3:0] of BG12NBA.
    #[test]
    fn test_bg1_tiledata_addr_derivation() {
        let mut ppu = PPU::new();
        ppu.write(0x210B, 0x01); // nibble = 1 -> 0x1000
        assert_eq!(ppu.regs.bg1_tiledata_addr(), 0x1000);
    }

    /// bg1_tiledata_addr must return 0 when BG12NBA is 0.
    #[test]
    fn test_bg1_tiledata_addr_zero() {
        let mut ppu = PPU::new();
        ppu.write(0x210B, 0x00);
        assert_eq!(ppu.regs.bg1_tiledata_addr(), 0x0000);
    }

    /// bg1_tiledata_addr must handle the maximum low nibble (0xF -> 0xF000).
    #[test]
    fn test_bg1_tiledata_addr_maximum() {
        let mut ppu = PPU::new();
        ppu.write(0x210B, 0x0F);
        assert_eq!(ppu.regs.bg1_tiledata_addr(), 0xF000);
    }

    // ============================================================
    // $210D - BG1HOFS (two-write latch)
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

    /// A third write to $210D must start a new latch cycle without committing.
    #[test]
    fn test_bg1hofs_third_write_starts_new_cycle() {
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0x11);
        ppu.write(0x210D, 0x02); // commits 0x0211
        ppu.write(0x210D, 0x33); // new lo, not committed yet
        assert_eq!(ppu.regs.bg1hofs, 0x0211);
    }

    // ============================================================
    // $210E - BG1VOFS (two-write latch)
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
    // $210F–$2114 - remaining BG scroll (placeholder writes)
    // ============================================================

    /// Writing $210F must update bg1vofs as a raw u8->u16.
    #[test]
    fn test_write_bg1vofs_placeholder() {
        let mut ppu = PPU::new();
        ppu.write(0x210F, 0x42);
        assert_eq!(ppu.regs.bg1vofs, 0x42);
    }

    /// Writing $2110 must update m7vofs.
    #[test]
    fn test_write_m7vofs() {
        let mut ppu = PPU::new();
        ppu.write(0x2110, 0x11);
        assert_eq!(ppu.regs.m7vofs, 0x11);
    }

    /// Writing $2111 must update bg2hofs.
    #[test]
    fn test_write_bg2hofs() {
        let mut ppu = PPU::new();
        ppu.write(0x2111, 0x22);
        assert_eq!(ppu.regs.bg2hofs, 0x22);
    }

    /// Writing $2112 must update bg2vofs.
    #[test]
    fn test_write_bg2vofs() {
        let mut ppu = PPU::new();
        ppu.write(0x2112, 0x33);
        assert_eq!(ppu.regs.bg2vofs, 0x33);
    }

    /// Writing $2113 must update bg3hofs.
    #[test]
    fn test_write_bg3hofs() {
        let mut ppu = PPU::new();
        ppu.write(0x2113, 0x44);
        assert_eq!(ppu.regs.bg3hofs, 0x44);
    }

    /// Writing $2114 must update bg3vofs.
    #[test]
    fn test_write_bg3vofs() {
        let mut ppu = PPU::new();
        ppu.write(0x2114, 0x55);
        assert_eq!(ppu.regs.bg3vofs, 0x55);
    }

    // ============================================================
    // $2115–$2119 - VRAM / $2139–$213A - VRAM read
    // ============================================================

    /// Writing $2115 must update vmain.
    #[test]
    fn test_write_vmain() {
        let mut ppu = PPU::new();
        ppu.write(0x2115, 0x80);
        assert_eq!(ppu.regs.vmain, 0x80);
    }

    /// Setting VRAM address and writing a word must store the correct data.
    #[test]
    fn test_vram_write_via_ppu() {
        let mut ppu = PPU::new();
        ppu.write(0x2115, 0x80); // increment after high byte
        ppu.write(0x2116, 0x10);
        ppu.write(0x2117, 0x00); // address = 0x0010
        ppu.write(0x2118, 0xCD);
        ppu.write(0x2119, 0xAB); // word = 0xABCD
        assert_eq!(ppu.vram.memory[0x0010], 0xABCD);
    }

    /// Reading $2139/$213A must return the previously written VRAM data.
    #[test]
    fn test_vram_read_via_ppu() {
        let mut ppu = PPU::new();
        ppu.vram.memory[0x0005] = 0x1234;
        ppu.write(0x2115, 0x80);
        ppu.write(0x2116, 0x05);
        ppu.write(0x2117, 0x00);
        let lo = ppu.read(0x2139);
        let hi = ppu.read(0x213A);
        assert_eq!(lo, 0x34);
        assert_eq!(hi, 0x12);
    }

    /// VRAM address must increment after a complete word write (vmain = 0x80).
    #[test]
    fn test_vram_address_increments_after_write() {
        let mut ppu = PPU::new();
        ppu.write(0x2115, 0x80);
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
    // $211A–$2120 - Mode 7
    // ============================================================

    /// Writing $211A must update m7sel.
    #[test]
    fn test_write_m7sel() {
        let mut ppu = PPU::new();
        ppu.write(0x211A, 0x03);
        assert_eq!(ppu.regs.m7sel, 0x03);
    }

    /// Writing $211B must update m7a.
    #[test]
    fn test_write_m7a() {
        let mut ppu = PPU::new();
        ppu.write(0x211B, 0x7F);
        assert_eq!(ppu.regs.m7a, 0x7F);
    }

    /// Writing $211C must update m7b.
    #[test]
    fn test_write_m7b() {
        let mut ppu = PPU::new();
        ppu.write(0x211C, 0x01);
        assert_eq!(ppu.regs.m7b, 0x01);
    }

    /// Writing $211D must update m7c.
    #[test]
    fn test_write_m7c() {
        let mut ppu = PPU::new();
        ppu.write(0x211D, 0x02);
        assert_eq!(ppu.regs.m7c, 0x02);
    }

    /// Writing $211E must update m7d.
    #[test]
    fn test_write_m7d() {
        let mut ppu = PPU::new();
        ppu.write(0x211E, 0x03);
        assert_eq!(ppu.regs.m7d, 0x03);
    }

    /// Writing $211F must update m7x.
    #[test]
    fn test_write_m7x() {
        let mut ppu = PPU::new();
        ppu.write(0x211F, 0x80);
        assert_eq!(ppu.regs.m7x, 0x80);
    }

    /// Writing $2120 must update m7y.
    #[test]
    fn test_write_m7y() {
        let mut ppu = PPU::new();
        ppu.write(0x2120, 0x40);
        assert_eq!(ppu.regs.m7y, 0x40);
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
            assert_eq!(getter(&ppu.regs), 0xA5, "register at ${:04X} did not store value", addr);
        }
    }

    // ============================================================
    // $212C–$2132 - Color math / layer enable
    // ============================================================

    /// Writing $212C must update tm.
    #[test]
    fn test_write_tm() {
        let mut ppu = PPU::new();
        ppu.write(0x212C, 0x1F);
        assert_eq!(ppu.regs.tm, 0x1F);
    }

    /// Writing color math registers must store the value verbatim.
    #[test]
    fn test_write_color_math_registers() {
        let cases: &[(u16, fn(&PPURegisters) -> u8)] = &[
            (0x212D, |r| r.ts),
            (0x212E, |r| r.tmw),
            (0x212F, |r| r.tsw),
            (0x2130, |r| r.cgwsel),
            (0x2131, |r| r.cgadsub),
            (0x2132, |r| r.coldata),
        ];
        for &(addr, getter) in cases {
            let mut ppu = PPU::new();
            ppu.write(addr, 0xA5);
            assert_eq!(getter(&ppu.regs), 0xA5, "register at ${:04X} did not store value", addr);
        }
    }

    /// Writing $2133 must update setini.
    #[test]
    fn test_write_setini() {
        let mut ppu = PPU::new();
        ppu.write(0x2133, 0x04);
        assert_eq!(ppu.regs.setini, 0x04);
    }

    // ============================================================
    // $212C - TM / bg1_enabled() / bg1_tilemap_addr()
    // ============================================================

    /// bg1_enabled must return true when bit 0 of TM is set.
    #[test]
    fn test_bg1_enabled_true_when_tm_bit0_set() {
        let mut ppu = PPU::new();
        ppu.write(0x212C, 0x01);
        assert!(ppu.regs.bg1_enabled());
    }

    /// bg1_enabled must return false when bit 0 of TM is clear.
    #[test]
    fn test_bg1_enabled_false_when_tm_bit0_clear() {
        let mut ppu = PPU::new();
        ppu.write(0x212C, 0xFE);
        assert!(!ppu.regs.bg1_enabled());
    }

    /// bg1_enabled must ignore all bits of TM except bit 0.
    #[test]
    fn test_bg1_enabled_ignores_upper_bits_of_tm() {
        let mut ppu = PPU::new();
        ppu.write(0x212C, 0x1E);
        assert!(!ppu.regs.bg1_enabled());
    }

    /// bg1_tilemap_addr must derive the VRAM address from bits[7:2] of BG1SC.
    #[test]
    fn test_bg1_tilemap_addr_derivation() {
        let mut ppu = PPU::new();
        ppu.write(0x2107, 0b00000100); // bits[7:2] = 1 -> 0x0400
        assert_eq!(ppu.regs.bg1_tilemap_addr(), 0x0400);
    }

    /// bg1_tilemap_addr must return 0 when BG1SC is 0.
    #[test]
    fn test_bg1_tilemap_addr_zero() {
        let mut ppu = PPU::new();
        ppu.write(0x2107, 0x00);
        assert_eq!(ppu.regs.bg1_tilemap_addr(), 0x0000);
    }

    /// bg1_tilemap_addr must handle the maximum value (bits[7:2] = 0x3F -> 0x3F * 0x400).
    #[test]
    fn test_bg1_tilemap_addr_maximum() {
        let mut ppu = PPU::new();
        ppu.write(0x2107, 0xFF);
        assert_eq!(ppu.regs.bg1_tilemap_addr(), 0x3F * 0x400);
    }

    // ============================================================
    // Read-only registers - must return 0 and not panic
    // ============================================================

    /// Reading unimplemented read-only registers must return 0 and not panic.
    #[test]
    fn test_read_unimplemented_read_only_registers() {
        let mut ppu = PPU::new();
        let read_only: &[u16] = &[
            0x2134, // MPYL
            0x2135, // MPYM
            0x2136, // MPYH
            0x2137, // SLHV
            0x2138, // OAMDATAREAD
            0x213C, // OPHCT
            0x213D, // OPVCT
            0x213E, // STAT77
            0x213F, // STAT78
        ];
        for &addr in read_only {
            assert_eq!(ppu.read(addr), 0, "read at ${:04X} must return 0", addr);
        }
    }

    // ============================================================
    // Unhandled addresses
    // ============================================================

    /// Writing to an unimplemented address must not panic or corrupt any state.
    #[test]
    fn test_write_unknown_address_does_not_panic() {
        let mut ppu = PPU::new();
        ppu.write(0xFFFF, 0x42);
        assert_eq!(ppu.regs.inidisp, 0);
    }

    /// Reading from an unimplemented address must return 0 and not panic.
    #[test]
    fn test_read_unknown_address_returns_zero() {
        let mut ppu = PPU::new();
        assert_eq!(ppu.read(0xFFFF), 0);
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

    /// After SCANLINES_PER_FRAME steps, scanline must wrap to 0 and frame_ready be set.
    #[test]
    fn test_step_scanline_wraps_at_262() {
        let mut ppu = PPU::new();
        for _ in 0..SCANLINES_PER_FRAME {
            ppu.step_scanline();
        }
        assert_eq!(ppu.scanline, 0);
        assert!(ppu.frame_ready);
    }

    /// frame_ready must remain false until the last scanline is reached.
    #[test]
    fn test_frame_ready_false_before_wrap() {
        let mut ppu = PPU::new();
        for _ in 0..SCANLINES_PER_FRAME - 1 {
            ppu.step_scanline();
        }
        assert!(!ppu.frame_ready);
        assert_eq!(ppu.scanline, SCANLINES_PER_FRAME - 1);
    }

    /// frame_ready must remain true across subsequent frames.
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

use crate::write_twice::WriteTwice;

/// PPU Registers placeholder definitions
/// Each field is a placeholder; actual behavior, latches, buffering, and timing to implement later.
pub struct PPURegisters {
    // $2100 - INIDISP
    pub inidisp: u8, // Bits: F...BBBB | Forced blanking (F), screen brightness (B).

    // $2101 - OBJSEL
    pub objsel: u8, // Bits: SSSNNbBB | OBJ sprite size (S), name secondary select (N), name base address (B).

    // $2102 - OAMADDL
    pub oamaddl: u8, // Bits: AAAAAAAA | OAM word address low

    // $2103 - OAMADDH
    pub oamaddh: u8, // Bits: P...B | Priority rotation (P), address high bit (B)

    // $2104 - OAMDATA
    pub oamdata: u8, // Bits: DDDDDDDD | OAM data write byte, increments OAMADD

    // $2105 - BGMODE
    pub bgmode: u8, // Bits: 4321PMMM | Tilemap tile size (#), BG3 priority (P), BG mode (M)

    // $2106 - MOSAIC
    pub mosaic: u8, // Bits: SSSS4321 | Mosaic size (S), mosaic BG enable (#)

    // $2107 - BG1SC
    pub bg1sc: u8, // Bits: AAAAAAYX | Tilemap VRAM address (A), vertical/horizontal tilemap count

    // $2108 - BG2SC
    pub bg2sc: u8, // Bits: AAAAAAYX | Tilemap VRAM address (A), vertical/horizontal tilemap count

    // $2109 - BG3SC
    pub bg3sc: u8, // Bits: AAAAAAYX | Tilemap VRAM address (A), vertical/horizontal tilemap count

    // $210A - BG4SC
    pub bg4sc: u8, // Bits: AAAAAAYX | Tilemap VRAM address (A), vertical/horizontal tilemap count

    // $210B - BG12NBA
    pub bg12nba: u8, // Bits: BBBBAAAA | BG2 CHR base address (B), BG1 CHR base address (A)

    // $210C - BG34NBA
    pub bg34nba: u8, // Bits: DDDDCCCC | BG4 CHR base address (D), BG3 CHR base address (C)

    // $210D - BG1HOFS
    pub bg1hofs: u16, // Bits: .... ..XX XXXX XXXX | BG1 horizontal scroll (X)

    // $210E - M7HOFS
    pub m7hofs: u16, // Bits: .... ..XX XXXX XXXX | Mode 7 horizontal scroll (x)

    // $210F - BG1VOFS
    pub bg1vofs: u16, // Bits: .... ..YY YYYY YYYY | BG1 vertical scroll (Y)

    // $2110 - M7VOFS
    pub m7vofs: u16, // Bits: .... ..YY YYYY YYYY | Mode 7 vertical scroll (y)

    // $2111 - BG2HOFS
    pub bg2hofs: u16, // Bits: .... ..XX XXXX XXXX | BG2 horizontal scroll (X)

    // $2112 - BG2VOFS
    pub bg2vofs: u16, // Bits: .... ..YY YYYY YYYY | BG2 vertical scroll (Y)

    // $2113 - BG3HOFS
    pub bg3hofs: u16, // Bits: .... ..XX XXXX XXXX | BG3 horizontal scroll (X)

    // $2114 - BG3VOFS
    pub bg3vofs: u16, // Bits: .... ..YY YYYY YYYY | BG3 vertical scroll (Y)

    // $2115 - VMAIN
    pub vmain: u8, // Bits: M...RRII | VRAM address increment mode (M), remapping (R), increment size (I)

    // $2116 - VMADDL
    pub vmaddl: u8, // Bits: LLLLLLLL | VRAM word address low

    // $2117 - VMADDH
    pub vmaddh: u8, // Bits: hHHHHHHH | VRAM word address high

    // $2118 - VMDATAL
    pub vmdatal: u8, // Bits: LLLLLLLL | VRAM data write low, increments VMADD

    // $2119 - VMDATAH
    pub vmdatab: u8, // Bits: HHHHHHHH | VRAM data write high, increments VMADD

    // $211A - M7SEL
    pub m7sel: u8, // Bits: RF..YX | Mode 7 tilemap repeat (R), fill (F), flip vertical (Y), flip horizontal (X)

    // $211B - M7A
    pub m7a: u16, // Mode 7 matrix A

    // $211C - M7B
    pub m7b: u16, // Mode 7 matrix B

    // $211D - M7C
    pub m7c: u16, // Mode 7 matrix C

    // $211E - M7D
    pub m7d: u16, // Mode 7 matrix D

    // $211F - M7X
    pub m7x: u16, // Mode 7 center X

    // $2120 - M7Y
    pub m7y: u16, // Mode 7 center Y

    // $2121 - CGADD
    pub cgadd: u8, // CGRAM word address

    // $2122 - CGDATA
    pub cgdata: u16, // CGRAM data write, increments CGADD

    // $2123 - W12SEL
    pub w12sel: u8, // Enable/invert windows for BG1/BG2

    // $2124 - W34SEL
    pub w34sel: u8, // Enable/invert windows for BG3/BG4

    // $2125 - WOBJSEL
    pub wobjsel: u8, // Enable/invert windows for OBJ

    // $2126 - WH0
    pub wh0: u8, // Window 1 left position

    // $2127 - WH1
    pub wh1: u8, // Window 1 right position

    // $2128 - WH2
    pub wh2: u8, // Window 2 left position

    // $2129 - WH3
    pub wh3: u8, // Window 2 right position

    // $212A - WBGLOG
    pub wbglog: u8, // Window mask logic for BG layers

    // $212B - WOBJLOG
    pub wobjlog: u8, // Window mask logic for OBJ and color

    // $212C - TM
    pub tm: u8, // Main screen layer enable

    // $212D - TS
    pub ts: u8, // Sub screen layer enable

    // $212E - TMW
    pub tmw: u8, // Main screen layer window enable

    // $212F - TSW
    pub tsw: u8, // Sub screen layer window enable

    // $2130 - CGWSEL
    pub cgwsel: u8, // Main/sub screen color window, fixed/subscreen, direct color

    // $2131 - CGADSUB
    pub cgadsub: u8, // Color math add/subtract, half, backdrop, layer enable

    // $2132 - COLDATA
    pub coldata: u8, // Fixed color channel select (BGR) and value

    // $2133 - SETINI
    pub setini: u8, // External sync, EXTBG, Hi-res, Overscan, OBJ interlace, Screen interlace

    // $2134 - MPYL
    pub mpyl: u8, // Multiplication result low byte

    // $2135 - MPYM
    pub mpym: u8, // Multiplication result middle byte

    // $2136 - MPYH
    pub mpyh: u8, // Multiplication result high byte

    // $2137 - SLHV
    pub slhv: u8, // Software latch for H/V counters

    // $2138 - OAMDATAREAD
    pub oamdataread: u8, // Read OAM data byte

    // $2139 - VMDATALREAD
    pub vmdatalread: u8, // VRAM data read low

    // $213A - VMDATAHREAD
    pub vmdatahread: u8, // VRAM data read high

    // $213B - CGDATAREAD
    pub cgdataread: u16, // CGRAM data read

    // $213C - OPHCT
    pub ophct: u16, // Output horizontal counter

    // $213D - OPVCT
    pub opvct: u16, // Output vertical counter

    // $213E - STAT77
    pub stat77: u8, // Sprite overflow, sprite tile overflow, master/slave, PPU1 version

    // $213F - STAT78
    pub stat78: u8, // Interlace field, counter latch, NTSC/PAL, PPU2 version

    // Latches
    pub bg1hofs_latch: WriteTwice,
    pub bg1vofs_latch: WriteTwice,
    pub cgdata_latch: WriteTwice,
}

impl PPURegisters {
    pub fn new() -> Self {
        Self {
            inidisp: 0,
            objsel: 0,
            oamaddl: 0,
            oamaddh: 0,
            oamdata: 0,
            bgmode: 0,
            mosaic: 0,
            bg1sc: 0,
            bg2sc: 0,
            bg3sc: 0,
            bg4sc: 0,
            bg12nba: 0,
            bg34nba: 0,
            bg1hofs: 0,
            m7hofs: 0,
            bg1vofs: 0,
            m7vofs: 0,
            bg2hofs: 0,
            bg2vofs: 0,
            bg3hofs: 0,
            bg3vofs: 0,
            vmain: 0,
            vmaddl: 0,
            vmaddh: 0,
            vmdatal: 0,
            vmdatab: 0,
            m7sel: 0,
            m7a: 0,
            m7b: 0,
            m7c: 0,
            m7d: 0,
            m7x: 0,
            m7y: 0,
            cgadd: 0,
            cgdata: 0,
            w12sel: 0,
            w34sel: 0,
            wobjsel: 0,
            wh0: 0,
            wh1: 0,
            wh2: 0,
            wh3: 0,
            wbglog: 0,
            wobjlog: 0,
            tm: 0,
            ts: 0,
            tmw: 0,
            tsw: 0,
            cgwsel: 0,
            cgadsub: 0,
            coldata: 0,
            setini: 0,
            mpyl: 0,
            mpym: 0,
            mpyh: 0,
            slhv: 0,
            oamdataread: 0,
            vmdatalread: 0,
            vmdatahread: 0,
            cgdataread: 0,
            ophct: 0,
            opvct: 0,
            stat77: 0,
            stat78: 0,
            bg1hofs_latch: WriteTwice::new(),
            bg1vofs_latch: WriteTwice::new(),
            cgdata_latch: WriteTwice::new(),
        }
    }

    // ============================================================
    // Helpers
    // ============================================================

    pub fn bg1_enabled(&self) -> bool {
        (self.tm & 0x01) != 0
    }

    pub fn bg_mode(&self) -> u8 {
        self.bgmode & 0x07
    }

    pub fn bg1_tilemap_addr(&self) -> u16 {
        (self.bg1sc as u16 >> 2) * 0x400
    }

    pub fn bg1_tiledata_addr(&self) -> u16 {
        (self.bg12nba as u16) << 12
    }
}

#[cfg(test)]
mod tests {
    use crate::ppu::PPU;

    // ============================================================
    // PPU::new - initial state
    // ============================================================

    /// All registers must be zeroed after construction.
    #[test]
    fn test_new_all_registers_zeroed() {
        let ppu = PPU::new();
        assert_eq!(ppu.regs.inidisp, 0);
        assert_eq!(ppu.regs.bgmode, 0);
        assert_eq!(ppu.regs.vmain, 0);
        assert_eq!(ppu.regs.tm, 0);
        assert_eq!(ppu.regs.bg1hofs, 0);
        assert_eq!(ppu.regs.bg1vofs, 0);
        assert_eq!(ppu.scanline, 0);
        assert!(!ppu.frame_ready);
    }

    // ============================================================
    // $2100 - INIDISP
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

    /// brightness must return only bits[3:0] of INIDISP.
    #[test]
    fn test_brightness_returns_lower_nibble() {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0xFF);
        assert_eq!(ppu.brightness(), 0x0F);
    }

    /// brightness must return 0 when the lower nibble of INIDISP is 0.
    #[test]
    fn test_brightness_zero_when_lower_nibble_clear() {
        let mut ppu = PPU::new();
        ppu.write(0x2100, 0x80);
        assert_eq!(ppu.brightness(), 0x00);
    }

    // ============================================================
    // $2105 - BGMODE
    // ============================================================

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
    // $2107 - BG1SC
    // ============================================================

    /// bg1_tilemap_addr must derive the VRAM address from bits[7:2] of BG1SC.
    #[test]
    fn test_bg1_tilemap_addr_derivation() {
        let mut ppu = PPU::new();
        // bits[7:2] = 1 -> 1 * 0x400 = 0x0400
        ppu.write(0x2107, 0b00000100);
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
    // $210B - BG12NBA
    // ============================================================

    /// bg1_tiledata_addr must derive the CHR base address from bits[3:0] of BG12NBA.
    #[test]
    fn test_bg1_tiledata_addr_derivation() {
        let mut ppu = PPU::new();
        // nibble = 1 -> 1 << 12 = 0x1000
        ppu.write(0x210B, 0x01);
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

    /// First write to $210D must latch the low byte without committing to bg1hofs.
    #[test]
    fn test_bg1hofs_first_write_only_latches() {
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0xAB);
        assert_eq!(ppu.regs.bg1hofs, 0x0000);
    }

    /// Second write to $210D must commit lo+hi into bg1hofs, masking hi to 3 bits.
    #[test]
    fn test_bg1hofs_second_write_commits_word() {
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0xCD); // lo
        ppu.write(0x210D, 0x03); // hi
        assert_eq!(ppu.regs.bg1hofs, 0x03CD);
    }

    /// The hi byte of bg1hofs must be masked to 3 bits (scroll is 10-bit on SNES).
    #[test]
    fn test_bg1hofs_hi_byte_masked_to_3_bits() {
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0xFF);
        ppu.write(0x210D, 0xFF); // only bits[2:0] must survive
        assert_eq!(ppu.regs.bg1hofs, 0x07FF);
    }

    /// A third write to $210D must start a new latch cycle, preserving the previous value.
    #[test]
    fn test_bg1hofs_third_write_starts_new_cycle() {
        let mut ppu = PPU::new();
        ppu.write(0x210D, 0x11);
        ppu.write(0x210D, 0x02);
        ppu.write(0x210D, 0x33); // new lo - not committed yet
        assert_eq!(ppu.regs.bg1hofs, 0x0211);
    }

    // ============================================================
    // $210E - BG1VOFS (two-write latch)
    // ============================================================

    /// First write to $210E must latch without committing.
    #[test]
    fn test_bg1vofs_first_write_only_latches() {
        let mut ppu = PPU::new();
        ppu.write(0x210E, 0xAB);
        assert_eq!(ppu.regs.bg1vofs, 0x0000);
    }

    /// Second write to $210E must commit lo+hi into bg1vofs, masking hi to 3 bits.
    #[test]
    fn test_bg1vofs_second_write_commits_word() {
        let mut ppu = PPU::new();
        ppu.write(0x210E, 0xCD);
        ppu.write(0x210E, 0x03);
        assert_eq!(ppu.regs.bg1vofs, 0x03CD);
    }

    /// The hi byte of bg1vofs must be masked to 3 bits.
    #[test]
    fn test_bg1vofs_hi_byte_masked_to_3_bits() {
        let mut ppu = PPU::new();
        ppu.write(0x210E, 0xFF);
        ppu.write(0x210E, 0xFF);
        assert_eq!(ppu.regs.bg1vofs, 0x07FF);
    }

    // ============================================================
    // $2115 - VMAIN / $2116-2117 - VMADD / $2118-2119 - VMDATA
    // ============================================================

    /// VMAIN must store the increment mode byte verbatim.
    #[test]
    fn test_vmain_stores_value() {
        let mut ppu = PPU::new();
        ppu.write(0x2115, 0x80); // increment after high byte access, step 1
        assert_eq!(ppu.regs.vmain, 0x80);
    }

    /// Writing to $2116/$2117 must update the VRAM word address low/high bytes.
    #[test]
    fn test_vmadd_low_high_store_address() {
        let mut ppu = PPU::new();
        ppu.write(0x2116, 0x34);
        ppu.write(0x2117, 0x12);
        assert_eq!(ppu.regs.vmaddl, 0x34);
        assert_eq!(ppu.regs.vmaddh, 0x12);
    }

    /// Writing a word to VRAM via $2118/$2119 must store data and advance VMADD.
    #[test]
    fn test_vmdatal_vmdatah_write_and_advance() {
        let mut ppu = PPU::new();
        ppu.write(0x2115, 0x00); // increment on low byte write, step 1
        ppu.write(0x2116, 0x00);
        ppu.write(0x2117, 0x00);
        ppu.write(0x2118, 0xAB); // low byte -> VMADD increments
        let addr_after_low = u16::from(ppu.regs.vmaddl) | (u16::from(ppu.regs.vmaddh) << 8);
        assert_eq!(addr_after_low, 0x0001);
        ppu.write(0x2116, 0x00);
        ppu.write(0x2117, 0x00);
        ppu.write(0x2115, 0x80); // increment on high byte write, step 1
        ppu.write(0x2119, 0xCD); // high byte -> VMADD increments
        let addr_after_high = u16::from(ppu.regs.vmaddl) | (u16::from(ppu.regs.vmaddh) << 8);
        assert_eq!(addr_after_high, 0x0001);
    }

    // ============================================================
    // $2139/$213A - VRAM read
    // ============================================================

    /// Data written to VRAM must be readable back via $2139/$213A.
    #[test]
    fn test_vram_read_back() {
        let mut ppu = PPU::new();
        ppu.write(0x2115, 0x00); // increment on low, step 1
        ppu.write(0x2116, 0x00);
        ppu.write(0x2117, 0x00);
        ppu.write(0x2118, 0xAB);
        ppu.write(0x2116, 0x00);
        ppu.write(0x2117, 0x00);
        ppu.write(0x2115, 0x80); // increment on high, step 1
        ppu.write(0x2119, 0xCD);

        ppu.write(0x2115, 0x00);
        ppu.write(0x2116, 0x00);
        ppu.write(0x2117, 0x00);
        assert_eq!(ppu.read(0x2139), 0xAB);
        ppu.write(0x2115, 0x80);
        ppu.write(0x2116, 0x00);
        ppu.write(0x2117, 0x00);
        assert_eq!(ppu.read(0x213A), 0xCD);
    }

    // ============================================================
    // $2121/$2122/$213B - CGRAM
    // ============================================================

    /// A color written to CGRAM must be readable back via $213B.
    #[test]
    fn test_cgram_write_and_read_back() {
        let mut ppu = PPU::new();
        ppu.write(0x2121, 0x00); // set CGRAM address to 0
        ppu.write(0x2122, 0x5A); // lo byte
        ppu.write(0x2122, 0x1F); // hi byte (only 7 bits used by SNES)
        ppu.write(0x2121, 0x00); // reset address for read
        let lo = ppu.read(0x213B);
        let hi = ppu.read(0x213B);
        assert_eq!(lo, 0x5A);
        assert_eq!(hi & 0x7F, 0x1F);
    }

    // ============================================================
    // $212C - TM (main screen layer enable)
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
        ppu.write(0x212C, 0x1E); // bits 1-4 set, bit 0 clear
        assert!(!ppu.regs.bg1_enabled());
    }

    // ============================================================
    // step_scanline
    // ============================================================

    /// Scanline counter must increment by 1 each step.
    #[test]
    fn test_step_scanline_increments() {
        let mut ppu = PPU::new();
        ppu.step_scanline();
        assert_eq!(ppu.scanline, 1);
    }

    /// frame_ready must be set when scanline wraps past SCANLINES_PER_FRAME.
    #[test]
    fn test_step_scanline_sets_frame_ready_at_wrap() {
        use crate::constants::SCANLINES_PER_FRAME;
        let mut ppu = PPU::new();
        for _ in 0..SCANLINES_PER_FRAME {
            ppu.step_scanline();
        }
        assert!(ppu.frame_ready);
        assert_eq!(ppu.scanline, 0);
    }

    /// frame_ready must be false before a full frame has elapsed.
    #[test]
    fn test_frame_ready_false_before_wrap() {
        use crate::constants::SCANLINES_PER_FRAME;
        let mut ppu = PPU::new();
        for _ in 0..SCANLINES_PER_FRAME - 1 {
            ppu.step_scanline();
        }
        assert!(!ppu.frame_ready);
    }

    // ============================================================
    // Unhandled addresses
    // ============================================================

    /// Writing to an unhandled address must not panic or corrupt any register.
    #[test]
    fn test_write_unhandled_address_does_not_panic() {
        let mut ppu = PPU::new();
        ppu.write(0xFFFF, 0xFF);
        assert_eq!(ppu.regs.inidisp, 0);
        assert_eq!(ppu.regs.bgmode, 0);
        assert_eq!(ppu.regs.vmain, 0);
    }

    /// Reading from an unhandled address must return 0 and not panic.
    #[test]
    fn test_read_unhandled_address_returns_zero() {
        let mut ppu = PPU::new();
        assert_eq!(ppu.read(0xFFFF), 0);
    }
}

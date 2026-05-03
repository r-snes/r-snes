use common::u16_split::U16Split;

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
    pub bg1hofs_latch: u8,
    pub cgdata_latch: u8,
    pub bg1hofs_latch_written: bool,
    pub cgdata_latch_written: bool,
    pub bg1vofs_latch: u8,
    pub bg1vofs_latch_written: bool,
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
            bg1hofs_latch: 0,
            cgdata_latch: 0,
            bg1hofs_latch_written: false,
            cgdata_latch_written: false,
            bg1vofs_latch: 0,
            bg1vofs_latch_written: false,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x2100 => self.inidisp = value,
            0x2101 => self.objsel = value,
            0x2102 => self.oamaddl = value,
            0x2103 => self.oamaddh = value,
            0x2104 => self.oamdata = value,
            0x2105 => self.bgmode = value,
            0x2106 => self.mosaic = value,
            0x2107 => self.bg1sc = value,
            0x2108 => self.bg2sc = value,
            0x2109 => self.bg3sc = value,
            0x210A => self.bg4sc = value,
            0x210B => self.bg12nba = value,
            0x210C => self.bg34nba = value,

            0x210D => {
                if !self.bg1hofs_latch_written {
                    self.bg1hofs_latch = value; // 1st write = low byte
                    self.bg1hofs_latch_written = true;
                } else {
                    *self.bg1hofs.lo_mut() = self.bg1hofs_latch;
                    *self.bg1hofs.hi_mut() = value; // 2nd write = high byte

                    self.bg1hofs_latch_written = false; // reset latch
                }
            }
            0x210E => self.m7hofs = value as u16,
            0x210F => self.bg1vofs = value as u16,
            0x2110 => self.m7vofs = value as u16,
            0x2111 => self.bg2hofs = value as u16,
            0x2112 => self.bg2vofs = value as u16,
            0x2113 => self.bg3hofs = value as u16,
            0x2114 => self.bg3vofs = value as u16,

            0x2115 => self.vmain = value,
            0x2116 => self.vmaddl = value,
            0x2117 => self.vmaddh = value,
            0x2118 => self.vmdatal = value,
            0x2119 => self.vmdatab = value,

            0x211A => self.m7sel = value,

            0x211B => self.m7a = value as u16,
            0x211C => self.m7b = value as u16,
            0x211D => self.m7c = value as u16,
            0x211E => self.m7d = value as u16,
            0x211F => self.m7x = value as u16,
            0x2120 => self.m7y = value as u16,

            0x2121 => self.cgadd = value,
            0x2122 => {
                if !self.cgdata_latch_written {
                    self.cgdata_latch = value; // 1st write = low byte
                    self.cgdata_latch_written = true;
                } else {
                    *self.cgdata.lo_mut() = self.cgdata_latch;
                    *self.cgdata.hi_mut() = value; // 2nd write = high byte

                    self.cgdata_latch_written = false; // reset latch
                }
            }

            0x2123 => self.w12sel = value,
            0x2124 => self.w34sel = value,
            0x2125 => self.wobjsel = value,

            0x2126 => self.wh0 = value,
            0x2127 => self.wh1 = value,
            0x2128 => self.wh2 = value,
            0x2129 => self.wh3 = value,

            0x212A => self.wbglog = value,
            0x212B => self.wobjlog = value,

            0x212C => self.tm = value,
            0x212D => self.ts = value,
            0x212E => self.tmw = value,
            0x212F => self.tsw = value,

            0x2130 => self.cgwsel = value,
            0x2131 => self.cgadsub = value,
            0x2132 => self.coldata = value,
            0x2133 => self.setini = value,

            0x2134 => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: MPYL multiplication result low)",
                    addr, value
                );
            }

            0x2135 => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: MPYM multiplication result middle)",
                    addr, value
                );
            }

            0x2136 => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: MPYH multiplication result high)",
                    addr, value
                );
            }

            0x2137 => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: SLHV latch H/V counters)",
                    addr, value
                );
            }

            0x2138 => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: OAMDATAREAD)",
                    addr, value
                );
            }

            0x2139 => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: VMDATALREAD)",
                    addr, value
                );
            }

            0x213A => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: VMDATAHREAD)",
                    addr, value
                );
            }

            0x213B => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: CGDATAREAD)",
                    addr, value
                );
            }

            0x213C => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: OPHCT horizontal counter)",
                    addr, value
                );
            }

            0x213D => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: OPVCT vertical counter)",
                    addr, value
                );
            }

            0x213E => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: STAT77 PPU1 status)",
                    addr, value
                );
            }

            0x213F => {
                println!(
                    "PPU WRITE IGNORED: ${:04X} = {:02X} (read-only: STAT78 PPU2 status)",
                    addr, value
                );
            }

            _ => {
                println!("PPU write to unknown address: {:04X}", addr);
            }
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
    use super::*;

    // ============================================================
    // PPURegisters::new
    // ============================================================

    /// A freshly created PPURegisters must have all fields zeroed.
    #[test]
    fn test_new_all_zeroed() {
        let regs = PPURegisters::new();
        assert_eq!(regs.inidisp, 0);
        assert_eq!(regs.bgmode, 0);
        assert_eq!(regs.vmain, 0);
        assert_eq!(regs.tm, 0);
        assert_eq!(regs.bg1hofs, 0);
        assert_eq!(regs.bg1vofs, 0);
        assert!(!regs.bg1hofs_latch_written);
        assert!(!regs.cgdata_latch_written);
        assert!(!regs.bg1vofs_latch_written);
    }

    // ============================================================
    // write — simple single-byte registers
    // ============================================================

    /// Writing to each simple write-only register must store the value verbatim.
    #[test]
    fn test_write_simple_registers() {
        let cases: &[(u16, fn(&PPURegisters) -> u8)] = &[
            (0x2100, |r| r.inidisp),
            (0x2101, |r| r.objsel),
            (0x2102, |r| r.oamaddl),
            (0x2103, |r| r.oamaddh),
            (0x2104, |r| r.oamdata),
            (0x2105, |r| r.bgmode),
            (0x2106, |r| r.mosaic),
            (0x2107, |r| r.bg1sc),
            (0x2108, |r| r.bg2sc),
            (0x2109, |r| r.bg3sc),
            (0x210A, |r| r.bg4sc),
            (0x210B, |r| r.bg12nba),
            (0x210C, |r| r.bg34nba),
            (0x2115, |r| r.vmain),
            (0x2116, |r| r.vmaddl),
            (0x2117, |r| r.vmaddh),
            (0x2118, |r| r.vmdatal),
            (0x2119, |r| r.vmdatab),
            (0x211A, |r| r.m7sel),
            (0x2123, |r| r.w12sel),
            (0x2124, |r| r.w34sel),
            (0x2125, |r| r.wobjsel),
            (0x2126, |r| r.wh0),
            (0x2127, |r| r.wh1),
            (0x2128, |r| r.wh2),
            (0x2129, |r| r.wh3),
            (0x212A, |r| r.wbglog),
            (0x212B, |r| r.wobjlog),
            (0x212C, |r| r.tm),
            (0x212D, |r| r.ts),
            (0x212E, |r| r.tmw),
            (0x212F, |r| r.tsw),
            (0x2130, |r| r.cgwsel),
            (0x2131, |r| r.cgadsub),
            (0x2132, |r| r.coldata),
            (0x2133, |r| r.setini),
            (0x2121, |r| r.cgadd),
        ];
        for &(addr, getter) in cases {
            let mut regs = PPURegisters::new();
            regs.write(addr, 0xA5);
            assert_eq!(getter(&regs), 0xA5, "register at ${:04X} did not store value", addr);
        }
    }

    /// Writing to Mode 7 single-byte registers must store the value as u16.
    #[test]
    fn test_write_mode7_registers() {
        let cases: &[(u16, fn(&PPURegisters) -> u16)] = &[
            (0x211B, |r| r.m7a),
            (0x211C, |r| r.m7b),
            (0x211D, |r| r.m7c),
            (0x211E, |r| r.m7d),
            (0x211F, |r| r.m7x),
            (0x2120, |r| r.m7y),
            (0x210E, |r| r.m7hofs),
            (0x210F, |r| r.bg1vofs),
            (0x2110, |r| r.m7vofs),
            (0x2111, |r| r.bg2hofs),
            (0x2112, |r| r.bg2vofs),
            (0x2113, |r| r.bg3hofs),
            (0x2114, |r| r.bg3vofs),
        ];
        for &(addr, getter) in cases {
            let mut regs = PPURegisters::new();
            regs.write(addr, 0x42);
            assert_eq!(getter(&regs), 0x42, "mode7 register at ${:04X} did not store value", addr);
        }
    }

    // ============================================================
    // write $210D — BG1HOFS (two-write latch)
    // ============================================================

    /// First write to $210D must latch the low byte without committing to bg1hofs.
    #[test]
    fn test_bg1hofs_first_write_only_latches() {
        let mut regs = PPURegisters::new();
        regs.write(0x210D, 0xAB);
        assert_eq!(regs.bg1hofs, 0x0000); // not committed yet
        assert!(regs.bg1hofs_latch_written);
    }

    /// Second write to $210D must commit lo+hi into bg1hofs and reset the latch flag.
    #[test]
    fn test_bg1hofs_second_write_commits_word() {
        let mut regs = PPURegisters::new();
        regs.write(0x210D, 0xCD); // lo
        regs.write(0x210D, 0x03); // hi
        assert_eq!(regs.bg1hofs, 0x03CD);
        assert!(!regs.bg1hofs_latch_written);
    }

    /// A third write to $210D must start a new latch cycle (lo phase again).
    #[test]
    fn test_bg1hofs_third_write_starts_new_cycle() {
        let mut regs = PPURegisters::new();
        regs.write(0x210D, 0x11);
        regs.write(0x210D, 0x22);
        regs.write(0x210D, 0x33); // new lo latch
        assert_eq!(regs.bg1hofs, 0x2211); // previous value unchanged
        assert!(regs.bg1hofs_latch_written);
    }

    // ============================================================
    // write $2122 — CGDATA (two-write latch)
    // ============================================================

    /// First write to $2122 must latch the low byte without committing to cgdata.
    #[test]
    fn test_cgdata_first_write_only_latches() {
        let mut regs = PPURegisters::new();
        regs.write(0x2122, 0x55);
        assert_eq!(regs.cgdata, 0x0000);
        assert!(regs.cgdata_latch_written);
    }

    /// Second write to $2122 must commit lo+hi into cgdata and reset the latch flag.
    #[test]
    fn test_cgdata_second_write_commits_word() {
        let mut regs = PPURegisters::new();
        regs.write(0x2122, 0xEF); // lo
        regs.write(0x2122, 0x1A); // hi
        assert_eq!(regs.cgdata, 0x1AEF);
        assert!(!regs.cgdata_latch_written);
    }

    /// A third write to $2122 must start a new latch cycle.
    #[test]
    fn test_cgdata_third_write_starts_new_cycle() {
        let mut regs = PPURegisters::new();
        regs.write(0x2122, 0xAA);
        regs.write(0x2122, 0xBB);
        regs.write(0x2122, 0xCC); // new lo latch
        assert_eq!(regs.cgdata, 0xBBAA);
        assert!(regs.cgdata_latch_written);
    }

    // ============================================================
    // write — read-only registers must be ignored
    // ============================================================

    /// Writing to read-only registers must not change any observable state.
    #[test]
    fn test_write_to_read_only_registers_is_ignored() {
        let mut regs = PPURegisters::new();
        let read_only: &[u16] = &[
            0x2134, 0x2135, 0x2136, 0x2137, 0x2138,
            0x2139, 0x213A, 0x213B, 0x213C, 0x213D,
            0x213E, 0x213F,
        ];
        for &addr in read_only {
            regs.write(addr, 0xFF); // must not panic or corrupt state
        }
        // Spot-check: none of the writable registers were affected
        assert_eq!(regs.inidisp, 0);
        assert_eq!(regs.bgmode, 0);
        assert_eq!(regs.vmain, 0);
    }

    // ============================================================
    // bg1_enabled
    // ============================================================

    /// bg1_enabled must return true when bit 0 of TM is set.
    #[test]
    fn test_bg1_enabled_true_when_tm_bit0_set() {
        let mut regs = PPURegisters::new();
        regs.write(0x212C, 0x01);
        assert!(regs.bg1_enabled());
    }

    /// bg1_enabled must return false when bit 0 of TM is clear.
    #[test]
    fn test_bg1_enabled_false_when_tm_bit0_clear() {
        let mut regs = PPURegisters::new();
        regs.write(0x212C, 0xFE); // all bits set except bit 0
        assert!(!regs.bg1_enabled());
    }

    /// bg1_enabled must ignore all bits of TM except bit 0.
    #[test]
    fn test_bg1_enabled_ignores_upper_bits_of_tm() {
        let mut regs = PPURegisters::new();
        regs.write(0x212C, 0xFF);
        assert!(regs.bg1_enabled());
        regs.write(0x212C, 0x1E); // bits 1-4 set, bit 0 clear
        assert!(!regs.bg1_enabled());
    }

    // ============================================================
    // bg_mode
    // ============================================================

    /// bg_mode must return only the lower 3 bits of BGMODE.
    #[test]
    fn test_bg_mode_returns_lower_3_bits() {
        let mut regs = PPURegisters::new();
        regs.write(0x2105, 0b11110111); // upper bits set, mode = 7
        assert_eq!(regs.bg_mode(), 7);
    }

    /// bg_mode must mask out bits above bit 2.
    #[test]
    fn test_bg_mode_masks_upper_bits() {
        let mut regs = PPURegisters::new();
        regs.write(0x2105, 0b11111000); // mode bits all zero
        assert_eq!(regs.bg_mode(), 0);
    }

    // ============================================================
    // bg1_tilemap_addr
    // ============================================================

    /// bg1_tilemap_addr must derive the VRAM word address from bits[7:2] of BG1SC.
    #[test]
    fn test_bg1_tilemap_addr_derivation() {
        let mut regs = PPURegisters::new();
        // BG1SC = 0b00000100 -> bits[7:2] = 1 -> addr = 1 * 0x400 = 0x0400
        regs.write(0x2107, 0b00000100);
        assert_eq!(regs.bg1_tilemap_addr(), 0x0400);
    }

    /// bg1_tilemap_addr must return 0 when BG1SC is 0.
    #[test]
    fn test_bg1_tilemap_addr_zero() {
        let mut regs = PPURegisters::new();
        regs.write(0x2107, 0x00);
        assert_eq!(regs.bg1_tilemap_addr(), 0x0000);
    }

    /// bg1_tilemap_addr must correctly handle the maximum value (0xFC -> 0x3F * 0x400).
    #[test]
    fn test_bg1_tilemap_addr_maximum() {
        let mut regs = PPURegisters::new();
        regs.write(0x2107, 0xFF); // bits[7:2] = 0x3F
        assert_eq!(regs.bg1_tilemap_addr(), 0x3F * 0x400);
    }

    // ============================================================
    // bg1_tiledata_addr
    // ============================================================

    /// bg1_tiledata_addr must derive the CHR base address from bits[3:0] of BG12NBA.
    #[test]
    fn test_bg1_tiledata_addr_derivation() {
        let mut regs = PPURegisters::new();
        // BG12NBA low nibble = 1 -> addr = 1 << 12 = 0x1000
        regs.write(0x210B, 0x01);
        assert_eq!(regs.bg1_tiledata_addr(), 0x1000);
    }

    /// bg1_tiledata_addr must return 0 when BG12NBA is 0.
    #[test]
    fn test_bg1_tiledata_addr_zero() {
        let mut regs = PPURegisters::new();
        regs.write(0x210B, 0x00);
        assert_eq!(regs.bg1_tiledata_addr(), 0x0000);
    }

    /// bg1_tiledata_addr must use the full BG12NBA byte shifted left by 12
    /// (BG1 uses bits[3:0], BG2 uses bits[7:4] — but the helper uses the full byte).
    #[test]
    fn test_bg1_tiledata_addr_maximum() {
        let mut regs = PPURegisters::new();
        regs.write(0x210B, 0x0F); // nibble = 0xF -> 0xF << 12 = 0xF000
        assert_eq!(regs.bg1_tiledata_addr(), 0xF000);
    }
}

//! PPU register file ($2100-$213F) and its internal latches.
//!
//! Holds the raw values of the write registers ($2100-$2133), the read
//! registers ($2134-$213F) and the hidden latches used by write-twice
//! registers (BG scroll, Mode 7, CGRAM, H/V counters).
//! Helpers decode the fields used by rendering (BG mode, tilemap and CHR addresses).

use crate::write_twice::WriteTwice;

/// PPU Registers placeholder definitions
/// Each field is a placeholder; actual behavior, latches, buffering, and timing to implement later.
// Registers without a field here:
// - $2104 OAMDATA, $2138 RDOAM: OAM ports, see oam.rs
// - $2118/$2119 VMDATA, $2139/$213A RDVRAM: VRAM ports, see vram.rs
// - $2121 CGADD, $2122 CGDATA, $213B RDCGRAM: CGRAM ports, see cgram.rs
// - $2137 SLHV, $213E STAT77, $213F STAT78: computed in PPU::read
pub struct PPURegisters {
    /// $2100 - INIDISP (W8)
    pub inidisp: u8, // Bits: F...BBBB | Forced blanking (F), screen brightness (B).

    /// $2101 - OBJSEL (W8)
    pub objsel: u8, // Bits: SSSNNbBB | OBJ sprite size (S), name secondary select (N), name base address (B).

    /// $2102/$2103 - OAMADDL/OAMADDH (W16)
    /// OAMADDL ($2102): Bits: AAAAAAAA | OAM word address low
    /// OAMADDH ($2103): Bits: P.......B | Priority rotation (P), address high bit (B)
    pub oamadd: u16,

    /// $2105 - BGMODE (W8)
    pub bgmode: u8, // Bits: 4321PMMM | Tilemap tile size (#), BG3 priority (P), BG mode (M)

    /// $2106 - MOSAIC (W8)
    pub mosaic: u8, // Bits: SSSS4321 | Mosaic size (S), mosaic BG enable (#)

    /// $2107/$2108/$2109/$210A - BG1SC/BG2SC/BG3SC/BG4SC (W8)
    /// Bits: AAAAAAYX | Tilemap VRAM address (A), vertical tilemap count (Y), horizontal tilemap count (X)
    /// bgsc[0] = BG1SC ($2107), bgsc[1] = BG2SC ($2108), bgsc[2] = BG3SC ($2109), bgsc[3] = BG4SC ($210A)
    pub bgsc: [u8; 4],

    /// $210B - BG12NBA (W8)
    pub bg12nba: u8, // Bits: BBBBAAAA | BG2 CHR base address (B), BG1 CHR base address (A)

    /// $210C - BG34NBA (W8)
    pub bg34nba: u8, // Bits: DDDDCCCC | BG4 CHR base address (D), BG3 CHR base address (C)

    /// $210D - BG1HOFS (W8x2, shares address with M7HOFS)
    /// Bits: ......XX XXXXXXXX | BG1 horizontal scroll
    /// On write: BG1HOFS = (value << 8) | (bgofs_latch & ~7) | (bghofs_latch & 7)
    ///           bgofs_latch = value; bghofs_latch = value
    pub bg1hofs: u16,

    /// $210D - M7HOFS (W8x2, shares address with BG1HOFS)
    /// Bits: ...XXXXX XXXXXXXX | Mode 7 horizontal scroll (signed)
    /// On write: M7HOFS = (value << 8) | mode7_latch; mode7_latch = value
    pub m7hofs: u16,

    /// $210E - BG1VOFS (W8x2, shares address with M7VOFS)
    /// Bits: ......YY YYYYYYYY | BG1 vertical scroll
    /// On write: BG1VOFS = (value << 8) | bgofs_latch; bgofs_latch = value
    pub bg1vofs: u16,

    /// $210E - M7VOFS (W8x2, shares address with BG1VOFS)
    /// Bits: ...YYYYY YYYYYYYY | Mode 7 vertical scroll (signed)
    /// On write: M7VOFS = (value << 8) | mode7_latch; mode7_latch = value
    pub m7vofs: u16,

    /// $210F/$2110/$2111/$2112/$2113/$2114 - BG2HOFS/BG2VOFS/BG3HOFS/BG3VOFS/BG4HOFS/BG4VOFS (W8x2)
    /// bghofs[0] = BG2HOFS ($210F), bghofs[1] = BG3HOFS ($2111), bghofs[2] = BG4HOFS ($2113)
    /// Bits: ......XX XXXXXXXX | BGn horizontal scroll
    /// On write: BGnHOFS = (value << 8) | (bgofs_latch & ~7) | (bghofs_latch & 7)
    ///           bgofs_latch = value; bghofs_latch = value
    pub bghofs: [u16; 3],

    /// bgvofs[0] = BG2VOFS ($2110), bgvofs[1] = BG3VOFS ($2112), bgvofs[2] = BG4VOFS ($2114)
    /// Bits: ......YY YYYYYYYY | BGn vertical scroll
    /// On write: BGnVOFS = (value << 8) | bgofs_latch; bgofs_latch = value
    pub bgvofs: [u16; 3],

    /// $2115 - VMAIN (W8)
    pub vmain: u8, // Bits: M...RRII | VRAM address increment mode (M), remapping (R), increment size (I)

    /// $2116/$2117 - VMADDL/VMADDH (W16)
    /// VMADDL ($2116): Bits: LLLLLLLL | VRAM word address low
    /// VMADDH ($2117): Bits: hHHHHHHH | VRAM word address high
    pub vmadd: u16,

    /// $211A - M7SEL (W8)
    pub m7sel: u8, // Bits: RF....YX | Mode 7 tilemap repeat (R), fill (F), flip vertical (Y), flip horizontal (X)

    /// $211B - M7A (W8x2)
    /// Bits: DDDDDDDD dddddddd | Mode 7 matrix A (8.8 fixed point) / 16-bit signed multiplication factor
    /// On write: M7A = (value << 8) | mode7_latch; mode7_latch = value
    pub m7a: u16,

    /// $211C - M7B (W8x2)
    /// Bits: DDDDDDDD dddddddd | Mode 7 matrix B (8.8 fixed point) / 8-bit signed multiplication factor
    /// On write: M7B = (value << 8) | mode7_latch; mode7_latch = value
    pub m7b: u16,

    /// $211D - M7C (W8x2)
    /// Bits: DDDDDDDD dddddddd | Mode 7 matrix C (8.8 fixed point)
    /// On write: M7C = (value << 8) | mode7_latch; mode7_latch = value
    pub m7c: u16,

    /// $211E - M7D (W8x2)
    /// Bits: DDDDDDDD dddddddd | Mode 7 matrix D (8.8 fixed point)
    /// On write: M7D = (value << 8) | mode7_latch; mode7_latch = value
    pub m7d: u16,

    /// $211F - M7X (W8x2)
    /// Bits: ...XXXXX XXXXXXXX | Mode 7 center X (signed)
    /// On write: M7X = (value << 8) | mode7_latch; mode7_latch = value
    pub m7x: u16,

    /// $2120 - M7Y (W8x2)
    /// Bits: ...YYYYY YYYYYYYY | Mode 7 center Y (signed)
    /// On write: M7Y = (value << 8) | mode7_latch; mode7_latch = value
    pub m7y: u16,

    /// $2123 - W12SEL (W8)
    pub w12sel: u8, // Bits: DdCcBbAa | Enable (ABCD) and invert (abcd) windows for BG1 (AB) and BG2 (CD)

    /// $2124 - W34SEL (W8)
    pub w34sel: u8, // Bits: HhGgFfEe | Enable (EFGH) and invert (efgh) windows for BG3 (EF) and BG4 (GH)

    /// $2125 - WOBJSEL (W8)
    pub wobjsel: u8, // Bits: LlKkJjIi | Enable (IJKL) and invert (ijkl) windows for OBJ (IJ) and color (KL)

    /// $2126 - WH0 (W8)
    pub wh0: u8, // Bits: LLLLLLLL | Window 1 left edge position

    /// $2127 - WH1 (W8)
    pub wh1: u8, // Bits: RRRRRRRR | Window 1 right edge position

    /// $2128 - WH2 (W8)
    pub wh2: u8, // Bits: LLLLLLLL | Window 2 left edge position

    /// $2129 - WH3 (W8)
    pub wh3: u8, // Bits: RRRRRRRR | Window 2 right edge position

    /// $212A - WBGLOG (W8)
    pub wbglog: u8, // Bits: 44332211 | Window mask logic for BG layers (00=OR, 01=AND, 10=XOR, 11=XNOR)

    /// $212B - WOBJLOG (W8)
    pub wobjlog: u8, // Bits: ....CCOO | Window mask logic for OBJ (O) and color (C)

    /// $212C - TM (W8)
    pub tm: u8, // Bits: ...O4321 | Main screen layer enable (OBJ, BG4-BG1)

    /// $212D - TS (W8)
    pub ts: u8, // Bits: ...O4321 | Sub screen layer enable (OBJ, BG4-BG1)

    /// $212E - TMW (W8)
    pub tmw: u8, // Bits: ...O4321 | Main screen layer window enable (OBJ, BG4-BG1)

    /// $212F - TSW (W8)
    pub tsw: u8, // Bits: ...O4321 | Sub screen layer window enable (OBJ, BG4-BG1)

    /// $2130 - CGWSEL (W8)
    pub cgwsel: u8, // Bits: MMSS..AD | Main/sub screen color window black/transparent (MS), fixed/subscreen (A), direct color (D)

    /// $2131 - CGADSUB (W8)
    pub cgadsub: u8, // Bits: MHBO4321 | Color math operator (M), half (H), backdrop (B), layer enable (O4321)

    /// $2132 - COLDATA (W8)
    pub coldata: u16, // Bits: BGRCCCCC | Fixed color channel select (BGR) and value (C). Accumulated into a BGR555 fixed color; also the sub-screen backdrop.

    /// $2133 - SETINI (W8)
    pub setini: u8, // Bits: EX..HOiI | External sync (E), EXTBG (X), Hi-res (H), Overscan (O), OBJ interlace (i), Screen interlace (I)

    /// $2134/$2135/$2136 - MPYL/MPYM/MPYH (R24, read-only)
    /// MPYL ($2134): Bits: LLLLLLLL | Multiplication result low byte
    /// MPYM ($2135): Bits: MMMMMMMM | Multiplication result middle byte
    /// MPYH ($2136): Bits: HHHHHHHH | Multiplication result high byte
    /// Signed 24-bit result of M7A (signed 16-bit) * M7B (signed 8-bit)
    pub mpy: u32,

    /// $213C - OPHCT (R8x2, read-only)
    /// Bits: xxxxxxxH HHHHHHHH | Output horizontal counter (9 bits)
    /// On read: if ophct_byte == 0: value = OPHCT.low
    ///          if ophct_byte == 1: value = OPHCT.high
    ///          ophct_byte = ~ophct_byte
    pub ophct: u16,

    /// $213D - OPVCT (R8x2, read-only)
    /// Bits: xxxxxxxV VVVVVVVV | Output vertical counter (9 bits)
    /// On read: if opvct_byte == 0: value = OPVCT.low
    ///          if opvct_byte == 1: value = OPVCT.high
    ///          opvct_byte = ~opvct_byte
    pub opvct: u16,

    /// ============================================================
    /// Latches (internal hardware state, not directly addressable)
    /// ============================================================

    /// Shared latch for all BGnHOFS/BGnVOFS writes ($210D-$2114).
    /// Written on every BGnHOFS and BGnVOFS write.
    pub bgofs_latch: u8,
    /// Second BG scroll latch, written on every BGnHOFS write only.
    /// Provides the low 3 bits of the next BGnHOFS value.
    pub bghofs_latch: u8,

    /// Shared latch for all Mode 7 writes ($210D-$210E, $211B-$2120).
    pub mode7_latch: u8,

    /// Internal flip-flop for CGDATA ($2122) and CGDATAREAD ($213B), shared per hardware.
    pub cgram_latch: WriteTwice,

    /// Internal flip-flop for OPHCT ($213C) reads.
    pub ophct_latch: WriteTwice,

    /// Internal flip-flop for OPVCT ($213D) reads.
    pub opvct_latch: WriteTwice,

    /// H/V counter latch flag: set by an SLHV ($2137) read, cleared by a STAT78 ($213F) read; shown in STAT78 bit 6.
    pub counter_latch: bool,
}

impl Default for PPURegisters {
    fn default() -> Self {
        Self::new()
    }
}

impl PPURegisters {
    /// Creates a register file with every register and latch cleared.
    pub fn new() -> Self {
        Self {
            inidisp: 0,
            objsel: 0,
            oamadd: 0,
            bgmode: 0,
            mosaic: 0,
            bgsc: [0; 4],
            bg12nba: 0,
            bg34nba: 0,
            bg1hofs: 0,
            m7hofs: 0,
            bg1vofs: 0,
            m7vofs: 0,
            bghofs: [0; 3],
            bgvofs: [0; 3],
            vmain: 0,
            vmadd: 0,
            m7sel: 0,
            m7a: 0,
            m7b: 0,
            m7c: 0,
            m7d: 0,
            m7x: 0,
            m7y: 0,
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
            mpy: 0,
            ophct: 0,
            opvct: 0,
            bgofs_latch: 0,
            bghofs_latch: 0,
            mode7_latch: 0,
            cgram_latch: WriteTwice::new(),
            ophct_latch: WriteTwice::new(),
            opvct_latch: WriteTwice::new(),
            counter_latch: false,
        }
    }

    // ============================================================
    // Helpers
    // ============================================================

    /// Returns true if BG1 is enabled on the main screen (TM bit 0).
    pub fn bg1_enabled(&self) -> bool {
        (self.tm & 0x01) != 0
    }

    /// Returns the current BG mode (BGMODE bits 0-2).
    pub fn bg_mode(&self) -> u8 {
        self.bgmode & 0x07
    }

    // ============================================================
    // Per-BG helpers
    // ============================================================

    /// Returns the tilemap word address of BG `bg` (BGnSC bits 2-6, 0x400-word steps).
    /// Bit 7 is ignored: VRAM only has 32K words.
    pub fn bg_tilemap_addr(&self, bg: usize) -> u16 {
        (((self.bgsc[bg] >> 2) & 0x1F) as u16) << 10
    }

    /// Returns the CHR word address of BG `bg` (BG12NBA/BG34NBA nibble bits 0-2, 0x1000-word steps).
    /// Bit 3 of the nibble is ignored: VRAM only has 32K words.
    pub fn bg_tiledata_addr(&self, bg: usize) -> u16 {
        let nib = match bg {
            0 => self.bg12nba & 0x0F,
            1 => self.bg12nba >> 4,
            2 => self.bg34nba & 0x0F,
            _ => self.bg34nba >> 4,
        };
        ((nib & 0x07) as u16) << 12
    }

    /// Returns the tilemap size of BG `bg` as (64 tiles wide, 64 tiles tall) (BGnSC bits 0-1).
    pub fn bg_tilemap_size(&self, bg: usize) -> (bool, bool) {
        (self.bgsc[bg] & 0x01 != 0, self.bgsc[bg] & 0x02 != 0)
    }

    /// Returns the (horizontal, vertical) scroll of BG `bg` (BGnHOFS / BGnVOFS).
    pub fn bg_scroll(&self, bg: usize) -> (usize, usize) {
        let h = if bg == 0 {
            self.bg1hofs
        } else {
            self.bghofs[bg - 1]
        };
        let v = if bg == 0 {
            self.bg1vofs
        } else {
            self.bgvofs[bg - 1]
        };
        (h as usize, v as usize)
    }

    /// Returns true if BG `bg` uses 16x16 tiles (BGMODE bits 4-7).
    pub fn bg_tile16(&self, bg: usize) -> bool {
        self.bgmode & (0x10 << bg) != 0
    }

    /// STAT78 ($213F) read side effects: clears the counter latch and resets the OPHCT/OPVCT toggles.
    pub fn read_stat78_side_effects(&mut self) {
        self.counter_latch = false;
        self.ophct_latch.reset();
        self.opvct_latch.reset();
    }
}

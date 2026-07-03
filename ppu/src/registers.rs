use crate::write_twice::WriteTwice;

/// PPU Registers placeholder definitions
/// Each field is a placeholder; actual behavior, latches, buffering, and timing to implement later.
pub struct PPURegisters {
    /// $2100 - INIDISP (W8)
    pub inidisp: u8, // Bits: F...BBBB | Forced blanking (F), screen brightness (B).

    /// $2101 - OBJSEL (W8)
    pub objsel: u8, // Bits: SSSNNbBB | OBJ sprite size (S), name secondary select (N), name base address (B).

    /// $2102/$2103 - OAMADDL/OAMADDH (W16)
    /// OAMADDL ($2102): Bits: AAAAAAAA | OAM word address low
    /// OAMADDH ($2103): Bits: P.......B | Priority rotation (P), address high bit (B)
    pub oamadd: u16,

    /// $2104 - OAMDATA (W8x2)
    pub oamdata: u8, // Bits: DDDDDDDD | OAM data write byte, increments OAMADD

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

    /// $2118/$2119 - VMDATAL/VMDATAH (W16)
    /// VMDATAL ($2118): Bits: LLLLLLLL | VRAM data write low
    /// VMDATAH ($2119): Bits: HHHHHHHH | VRAM data write high
    /// Increments VMADD after write according to VMAIN setting
    pub vmdata: u16,

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

    /// $2121 - CGADD (W8)
    pub cgadd: u8, // Bits: AAAAAAAA | CGRAM word address. On write: cgram_byte = 0

    /// $2122 - CGDATA (W8x2)
    /// Bits: .BBBBBGG GGGRRRRR | CGRAM data write, increments CGADD after each word write
    /// On write: if cgram_byte == 0: cgram_latch = value
    ///           if cgram_byte == 1: CGDATA = (value << 8) | cgram_latch
    ///           cgram_byte = ~cgram_byte
    pub cgdata: u16,

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
    pub coldata: u8, // Bits: BGRCCCCC | Fixed color channel select (BGR) and value (C)

    /// $2133 - SETINI (W8)
    pub setini: u8, // Bits: EX..HOiI | External sync (E), EXTBG (X), Hi-res (H), Overscan (O), OBJ interlace (i), Screen interlace (I)

    /// $2134/$2135/$2136 - MPYL/MPYM/MPYH (R24, read-only)
    /// MPYL ($2134): Bits: LLLLLLLL | Multiplication result low byte
    /// MPYM ($2135): Bits: MMMMMMMM | Multiplication result middle byte
    /// MPYH ($2136): Bits: HHHHHHHH | Multiplication result high byte
    /// Signed 24-bit result of M7A (signed 16-bit) * M7B (signed 8-bit)
    pub mpy: u32,

    /// $2137 - SLHV (R8, read-only)
    pub slhv: u8, // Bits: xxxxxxxx | CPU open bus. On read: counter_latch = 1

    /// $2138 - OAMDATAREAD (R8, read-only)
    pub oamdataread: u8, // Bits: DDDDDDDD | Read OAM data byte, increments OAMADD

    /// $2139/$213A - VMDATALREAD/VMDATAHREAD (R16, read-only)
    /// VMDATALREAD ($2139): Bits: LLLLLLLL | VRAM data read low (from vram_latch)
    /// VMDATAHREAD ($213A): Bits: HHHHHHHH | VRAM data read high (from vram_latch)
    /// Increments VMADD after read according to VMAIN setting
    pub vmdataread: u16,

    /// $213B - CGDATAREAD (R8x2, read-only)
    /// Bits: xBBBBBGG GGGRRRRR | CGRAM data read, increments CGADD after each word read
    /// On read: if cgram_byte == 0: value = CGDATA.low
    ///          if cgram_byte == 1: value = CGDATA.high
    ///          cgram_byte = ~cgram_byte
    pub cgdataread: u16,

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

    /// $213E - STAT77 (R8, read-only)
    pub stat77: u8, // Bits: TRMxVVVV | Time over/sprite overflow (T), range over/tile overflow (R), master/slave (M), PPU1 open bus (x), PPU1 version (V)

    /// $213F - STAT78 (R8, read-only)
    /// On read: counter_latch = 0; ophct_byte = 0; opvct_byte = 0
    pub stat78: u8, // Bits: FLxMVVVV | Interlace field (F), counter latch (L), PPU2 open bus (x), NTSC/PAL (M), PPU2 version (V)

    // ============================================================
    // Latches (internal hardware state, not directly addressable)
    // ============================================================

    /// Shared latch for all BGnHOFS/BGnVOFS writes ($210D-$2114)
    /// bgofs_latch is written on every BGnHOFS and BGnVOFS write
    /// bghofs_latch is written on every BGnHOFS write only
    pub bgofs_latch: u8,
    pub bghofs_latch: u8,

    // Shared latch for all Mode 7 writes ($210D-$2120, $211B-$211E)
    pub mode7_latch: u8,

    // Internal flip-flop for CGDATA ($2122) and CGDATAREAD ($213B) - shared per hardware
    pub cgram_latch: WriteTwice,

    // Internal flip-flop for OPHCT ($213C) reads
    pub ophct_latch: WriteTwice,

    // Internal flip-flop for OPVCT ($213D) reads
    pub opvct_latch: WriteTwice,
}

impl PPURegisters {
    pub fn new() -> Self {
        Self {
            inidisp: 0,
            objsel: 0,
            oamadd: 0,
            oamdata: 0,
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
            vmdata: 0,
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
            mpy: 0,
            slhv: 0,
            oamdataread: 0,
            vmdataread: 0,
            cgdataread: 0,
            ophct: 0,
            opvct: 0,
            stat77: 0,
            stat78: 0,
            bgofs_latch: 0,
            bghofs_latch: 0,
            mode7_latch: 0,
            cgram_latch: WriteTwice::new(),
            ophct_latch: WriteTwice::new(),
            opvct_latch: WriteTwice::new(),
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

    // Tilemap addresses - bgsc[n] bits[7:2] in 0x400-word steps
    pub fn bg1_tilemap_addr(&self) -> u16 {
        (self.bgsc[0] as u16 >> 2) * 0x400
    }

    pub fn bg2_tilemap_addr(&self) -> u16 {
        (self.bgsc[1] as u16 >> 2) * 0x400
    }

    pub fn bg3_tilemap_addr(&self) -> u16 {
        (self.bgsc[2] as u16 >> 2) * 0x400
    }

    pub fn bg4_tilemap_addr(&self) -> u16 {
        (self.bgsc[3] as u16 >> 2) * 0x400
    }

    // CHR base addresses - BG12NBA low/high nibble, BG34NBA low/high nibble, in 0x1000-word steps
    pub fn bg1_tiledata_addr(&self) -> u16 {
        (self.bg12nba as u16 & 0x0F) << 12
    }

    pub fn bg2_tiledata_addr(&self) -> u16 {
        (self.bg12nba as u16 >> 4) << 12
    }

    pub fn bg3_tiledata_addr(&self) -> u16 {
        (self.bg34nba as u16 & 0x0F) << 12
    }

    pub fn bg4_tiledata_addr(&self) -> u16 {
        (self.bg34nba as u16 >> 4) << 12
    }
}

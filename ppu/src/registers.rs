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

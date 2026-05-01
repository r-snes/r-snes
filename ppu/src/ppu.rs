use crate::registers::PPURegisters;
use crate::vram::VRAM;
use crate::cgram::CGRAM;
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

            // ==========================
            // BACKGROUNDS
            // ==========================
            0x2105 => self.regs.bgmode = value,
            0x2107 => self.regs.bg1sc = value,

            // BG1 HOFS
            0x210D => {
                if !self.regs.bg1hofs_latch_written {
                    self.regs.bg1hofs_latch = value;
                    self.regs.bg1hofs_latch_written = true;
                } else {
                    *self.regs.bg1hofs.lo_mut() = self.regs.bg1hofs_latch;
                    *self.regs.bg1hofs.hi_mut() = value & 0x07;

                    self.regs.bg1hofs_latch_written = false;
                }
            }

            // BG1 VOFS
            0x210E => {
                if !self.regs.bg1vofs_latch_written {
                    self.regs.bg1vofs_latch = value;
                    self.regs.bg1vofs_latch_written = true;
                } else {
                    *self.regs.bg1vofs.lo_mut() = self.regs.bg1vofs_latch;
                    *self.regs.bg1vofs.hi_mut() = value & 0x07;

                    self.regs.bg1vofs_latch_written = false;
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

        if self.scanline >= 262 {
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

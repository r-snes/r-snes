use crate::constants::{IO_END_ADDRESS, IO_START_ADDRESS};
use apu::Apu;
use common::snes_address::SnesAddress;
use cpu::cpu::CPU;
use ppu::ppu::PPU;

/// I/O Registers – 0x4000 bytes (mirrored)
///
/// - Memory area for various hardware components (CPU, APU, PPU, etc.).  
/// - Accessible in banks 0x00–0x3F and 0x80–0xBF, within the address
///   range 0x2000–0x5FFF.  
/// - Fully mirrored across all these banks.  
///
/// For example, the addresses `0x004000` and `0x9E4000` both refer to the
/// same memory location.
pub struct Io {
    pub nmitimen: u8,
    pub wrio: u8,

    pub wrmpya: u8,
    pub wrmpyb: u8,

    pub wrdivl: u8,
    pub wrdivh: u8,
    pub wrdivb: u8,

    pub htimel: u8,
    pub htimeh: u8,
    pub vtimel: u8,
    pub vtimeh: u8,

    pub mdmaen: u8,
    pub hdmaen: u8,
    pub memsel: u8,

    pub rddiv: u16,
    pub rdmpy: u16,

    pub rdnmi: u8,
    pub timeup: u8,
    pub hvbjoy: u8,

    pub joy1: u16,
    pub joy2: u16,
    pub joy3: u16,
    pub joy4: u16,

    pub dma_channels: [DMAChannel; 8],
}

#[derive(Copy, Clone)]
pub struct DMAChannel {
    pub dmap: u8,

    pub bbad: u8,

    pub a1tl: u8,
    pub a1th: u8,
    pub a1b: u8,

    pub dasl: u8,
    pub dash: u8,
    pub dasb: u8,

    pub a2al: u8,
    pub a2ah: u8,

    pub nltr: u8,

    pub unused: u8,
}

impl DMAChannel {
    pub fn new() -> Self {
        Self {
            dmap: 0xFF,

            bbad: 0xFF,

            a1tl: 0xFF,
            a1th: 0xFF,
            a1b: 0xFF,

            dasl: 0xFF,
            dash: 0xFF,
            dasb: 0xFF,

            a2al: 0xFF,
            a2ah: 0xFF,

            nltr: 0xFF,

            unused: 0xFF,
        }
    }
}

impl Io {
    pub fn new() -> Self {
        Self {
            nmitimen: 0,
            wrio: 0xFF,

            wrmpya: 0xFF,
            wrmpyb: 0xFF,

            wrdivl: 0xFF,
            wrdivh: 0xFF,
            wrdivb: 0xFF,

            htimel: 0xFF,
            htimeh: 1,
            vtimel: 0xFF,
            vtimeh: 1,

            mdmaen: 0,
            hdmaen: 0,
            memsel: 0,

            rddiv: 0,
            rdmpy: 0,

            rdnmi: 0,
            timeup: 0,
            hvbjoy: 0,

            joy1: 0,
            joy2: 0,
            joy3: 0,
            joy4: 0,

            dma_channels: [DMAChannel::new(); 8],
        }
    }

    fn panic_invalid_addr(addr: SnesAddress) -> ! {
        panic!(
            "Incorrect access to the IO at address: {:06X}",
            usize::from(addr)
        );
    }

    fn read_cpu(&mut self, addr: SnesAddress, cpu: &mut CPU, apu: &mut Apu) -> u8 {
        match addr.addr {
            // Data-from-APU register
            // TODO : Link with the actual apu component
            #[cfg(not(tarpaulin_include))]
            0x2140..0x2180 => {
                let reg_nb = addr.addr % 4;
                match reg_nb {
                    0 => todo!("{} : Implement APU channel n°1 reads", addr.addr),
                    1 => todo!("{} : Implement APU channel n°2 reads", addr.addr),
                    2 => todo!("{} : Implement APU channel n°3 reads", addr.addr),
                    3 => todo!("{} : Implement APU channel n°4 reads", addr.addr),
                    _ => unreachable!(),
                }
            }

            // S-WRAM Data Registers (Expansion port not implemented yet)
            #[cfg(not(tarpaulin_include))]
            0x2180 => todo!("0x2180-0x2183 : Implement Rom S-WRAM reads"),

            // JOYSER0/JOYSER1 - manual controller reading not implemented
            #[cfg(not(tarpaulin_include))]
            0x4016 => todo!("0x4016 : Implement JOYSER0 register read"),
            #[cfg(not(tarpaulin_include))]
            0x4017 => todo!("0x4017 : Implement JOYSER1 register read"),

            // Vblank flag and CPU version register
            // TODO : Implement open bus on unused bits
            0x4210 => {
                let value = self.rdnmi;
                self.rdnmi = self.rdnmi & 0x7F; // Reset V-Blank flag
                value
            }

            // Timer flag register
            // TODO : Implement open bus on unused bits
            0x4211 => {
                let value = self.timeup;
                self.timeup = self.timeup & 0x7F; // Reset Timer flag
                value
            }

            // Screen and Joypad status register
            // TODO : Implement open bus on unused bits
            0x4212 => self.hvbjoy,

            // RDIO : manual controller reading not implemented
            #[cfg(not(tarpaulin_include))]
            0x4213 => todo!("0x4213 : Implement RDIO register read"),

            // Divison result register
            0x4214 => self.rddiv as u8,
            0x4215 => (self.rddiv >> 8) as u8,

            // Multiplication result / Division remainder register
            0x4216 => self.rdmpy as u8,
            0x4217 => (self.rdmpy >> 8) as u8,

            // Joypad data registers
            0x4218 => self.joy1 as u8,
            0x4219 => (self.joy1 >> 8) as u8,
            0x421A => self.joy2 as u8,
            0x421B => (self.joy2 >> 8) as u8,
            0x421C => self.joy3 as u8,
            0x421D => (self.joy3 >> 8) as u8,
            0x421E => self.joy4 as u8,
            0x421F => (self.joy4 >> 8) as u8,

            // DMA and HDMA channel registers
            0x4300..0x4380 => {
                let channel_nb = (addr.addr - 0x4300) / 0x10;
                let reg_nb = (addr.addr - 0x4300) % 0x10;

                let channel = &mut self.dma_channels[channel_nb as usize];
                match reg_nb {
                    0x0 => channel.dmap,
                    0x1 => channel.bbad,
                    0x2 => channel.a1tl,
                    0x3 => channel.a1th,
                    0x4 => channel.a1b,
                    0x5 => channel.dasl,
                    0x6 => channel.dash,
                    0x7 => channel.dasb,
                    0x8 => channel.a2al,
                    0x9 => channel.a2ah,
                    0xA => channel.nltr,
                    0xB | 0xF => channel.unused,

                    _ => cpu.data_bus, // Open bus I believe, but not sure if this is the correct behavior
                }
            }

            // Open Bus
            _ => cpu.data_bus,
        }
    }

    fn write_cpu(&mut self, value: u8, addr: SnesAddress, cpu: &mut CPU, apu: &mut Apu) {
        match addr.addr {
            // Data-to-APU register
            #[cfg(not(tarpaulin_include))]
            0x2140..0x2180 => {
                let reg_nb = addr.addr % 4;
                match reg_nb {
                    0 => todo!("{} : Implement APU channel n°1 writes", addr.addr),
                    1 => todo!("{} : Implement APU channel n°2 writes", addr.addr),
                    2 => todo!("{} : Implement APU channel n°3 writes", addr.addr),
                    3 => todo!("{} : Implement APU channel n°4 writes", addr.addr),
                    _ => unreachable!(),
                }
            }

            // S-WRAM Data Registers (Expansion port not implemented yet)
            #[cfg(not(tarpaulin_include))]
            0x2180..=0x2183 => todo!("0x2180-0x2183 : Implement Rom S-WRAM writes"),

            // JOYOUT - manual controller reading not implemented
            #[cfg(not(tarpaulin_include))]
            0x4016 => todo!("0x4016 : Implement JOYOUT register write"),

            // Register for enabling NMI, H/V-Blank, and joypad auto-read
            0x4200 => self.nmitimen = value,

            // UNUSED : manual controller reading not implemented
            0x4201 => self.wrio = value,

            // Multiplication registers
            // TODO : Make the actual multiplication take 8 CPU cycles
            0x4202 => self.wrmpya = value,
            0x4203 => {
                self.wrmpyb = value;
                self.rdmpy = (self.wrmpya as u16) * (self.wrmpyb as u16);
            }

            // Division registers
            // TODO : Make the actual division take 16 CPU cycles
            0x4204 => self.wrdivl = value,
            0x4205 => self.wrdivh = value,
            0x4206 => {
                self.wrdivb = value;

                let dividend = ((self.wrdivh as u16) << 8) | self.wrdivl as u16;

                if value != 0 {
                    self.rddiv = dividend / value as u16;
                    self.rdmpy = dividend % value as u16;
                } else {
                    self.rddiv = 0xFFFF;
                    self.rdmpy = dividend;
                }
            }

            // Screen timer Horizontal target values
            0x4207 => self.htimel = value,
            0x4208 => self.htimeh = value,
            // Screen timer Vertical target values
            0x4209 => self.vtimel = value,
            0x420A => self.vtimeh = value,

            // DMA and HDMA enable registers
            // TODO : Implement real DMA and HDMA behaviors
            0x420B => self.mdmaen = value,
            0x420C => self.hdmaen = value,

            // ROM access speed register
            0x420D => self.memsel = value,

            // DMA and HDMA channel registers
            0x4300..0x4380 => {
                let channel_nb = (addr.addr - 0x4300) / 0x10;
                let reg_nb = (addr.addr - 0x4300) % 0x10;

                let channel = &mut self.dma_channels[channel_nb as usize];
                match reg_nb {
                    0x0 => channel.dmap = value,
                    0x1 => channel.bbad = value,
                    0x2 => channel.a1tl = value,
                    0x3 => channel.a1th = value,
                    0x4 => channel.a1b = value,
                    0x5 => channel.dasl = value,
                    0x6 => channel.dash = value,
                    0x7 => channel.dasb = value,
                    0x8 => channel.a2al = value,
                    0x9 => channel.a2ah = value,
                    0xA => channel.nltr = value,
                    0xB | 0xF => channel.unused = value,
                    _ => {}
                }
            }

            _ => {}
        }
    }

    #[cfg(not(tarpaulin_include))]
    fn read_ppu(&mut self, addr: SnesAddress, ppu: &mut PPU) -> u8 {
        match addr.addr {
            // MPY result (24-bit)
            0x2134 => todo!("0x2134 : MPYL read"),
            0x2135 => todo!("0x2135 : MPYM read"),
            0x2136 => todo!("0x2136 : MPYH read"),

            // Latch H/V counter
            0x2137 => todo!("0x2137 : SLHV read"),

            // OAM read
            0x2138 => todo!("0x2138 : OAMDATAREAD"),

            // VRAM read
            0x2139 => todo!("0x2139 : VMDATALREAD"),
            0x213A => todo!("0x213A : VMDATAHREAD"),

            // CGRAM read (2-step)
            0x213B => todo!("0x213B : CGDATAREAD"),

            // H/V counters (2-step reads)
            0x213C => todo!("0x213C : OPHCT read"),
            0x213D => todo!("0x213D : OPVCT read"),

            // Status registers
            0x213E => todo!("0x213E : STAT77 read"),
            0x213F => todo!("0x213F : STAT78 read"),

            // Open bus, may need to have a custom ppu open bus
            _ => 0,
        }
    }

    #[cfg(not(tarpaulin_include))]
    fn write_ppu(&mut self, value: u8, addr: SnesAddress, ppu: &mut PPU) {
        match addr.addr {
            // Display / OBJ
            0x2100 => todo!("0x2100 : INIDISP write"),
            0x2101 => todo!("0x2101 : OBJSEL write"),

            // OAM
            0x2102 => todo!("0x2102 : OAMADDL write"),
            0x2103 => todo!("0x2103 : OAMADDH write"),
            0x2104 => todo!("0x2104 : OAMDATA write"),

            // BG mode / mosaic
            0x2105 => todo!("0x2105 : BGMODE write"),
            0x2106 => todo!("0x2106 : MOSAIC write"),

            // BG tilemap
            0x2107 => todo!("0x2107 : BG1SC write"),
            0x2108 => todo!("0x2108 : BG2SC write"),
            0x2109 => todo!("0x2109 : BG3SC write"),
            0x210A => todo!("0x210A : BG4SC write"),

            // BG CHR base
            0x210B => todo!("0x210B : BG12NBA write"),
            0x210C => todo!("0x210C : BG34NBA write"),

            // Scroll registers (W8x2)
            0x210D => todo!("0x210D : BG1HOFS / M7HOFS write"),
            0x210E => todo!("0x210E : BG1VOFS / M7VOFS write"),
            0x210F => todo!("0x210F : BG2HOFS write"),
            0x2110 => todo!("0x2110 : BG2VOFS write"),
            0x2111 => todo!("0x2111 : BG3HOFS write"),
            0x2112 => todo!("0x2112 : BG3VOFS write"),
            0x2113 => todo!("0x2113 : BG4HOFS write"),
            0x2114 => todo!("0x2114 : BG4VOFS write"),

            // VRAM access
            0x2115 => todo!("0x2115 : VMAIN write"),
            0x2116 => todo!("0x2116 : VMADDL write"),
            0x2117 => todo!("0x2117 : VMADDH write"),
            0x2118 => todo!("0x2118 : VMDATAL write"),
            0x2119 => todo!("0x2119 : VMDATAH write"),

            // Mode 7
            0x211A => todo!("0x211A : M7SEL write"),
            0x211B => todo!("0x211B : M7A write"),
            0x211C => todo!("0x211C : M7B write"),
            0x211D => todo!("0x211D : M7C write"),
            0x211E => todo!("0x211E : M7D write"),
            0x211F => todo!("0x211F : M7X write"),
            0x2120 => todo!("0x2120 : M7Y write"),

            // CGRAM
            0x2121 => todo!("0x2121 : CGADD write"),
            0x2122 => todo!("0x2122 : CGDATA write"),

            // Window registers
            0x2123 => todo!("0x2123 : W12SEL write"),
            0x2124 => todo!("0x2124 : W34SEL write"),
            0x2125 => todo!("0x2125 : WOBJSEL write"),
            0x2126 => todo!("0x2126 : WH0 write"),
            0x2127 => todo!("0x2127 : WH1 write"),
            0x2128 => todo!("0x2128 : WH2 write"),
            0x2129 => todo!("0x2129 : WH3 write"),

            // Window logic
            0x212A => todo!("0x212A : WBGLOG write"),
            0x212B => todo!("0x212B : WOBJLOG write"),

            // Screen enable
            0x212C => todo!("0x212C : TM write"),
            0x212D => todo!("0x212D : TS write"),
            0x212E => todo!("0x212E : TMW write"),
            0x212F => todo!("0x212F : TSW write"),

            // Color math
            0x2130 => todo!("0x2130 : CGWSEL write"),
            0x2131 => todo!("0x2131 : CGADSUB write"),
            0x2132 => todo!("0x2132 : COLDATA write"),

            // Screen settings
            0x2133 => todo!("0x2133 : SETINI write"),

            _ => {}
        }
    }
}

impl Io {
    /// Reads a byte from the I/O memory zone at the given `SnesAddress`.
    ///
    /// The address is translated to an internal I/O offset using `to_offset`.
    ///
    /// # Panics
    /// Panics if the address does not map to a valid I/O memory location.
    pub fn read(&mut self, addr: SnesAddress, cpu: &mut CPU, ppu: &mut PPU, apu: &mut Apu) -> u8 {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF
                if addr.addr >= IO_START_ADDRESS && addr.addr < IO_END_ADDRESS =>
            {
                // 0x2000..0x6000 => self.io.DUP_method(DUP_method_param, cpu, ppu, apu),
                match addr.addr {
                    0x2000..0x2100 => cpu.data_bus,
                    #[cfg(not(tarpaulin_include))]
                    0x2100..0x2140 => self.read_ppu(addr, ppu),
                    0x2140..0x4380 => self.read_cpu(addr, cpu, apu),
                    0x4380..0x6000 => cpu.data_bus,

                    _ => Self::panic_invalid_addr(addr),
                }
            }
            _ => Self::panic_invalid_addr(addr),
        }
    }

    /// Writes a byte to the I/O memory zone at the given `SnesAddress`.
    ///
    /// The address is translated to an internal I/O offset using `to_offset`.
    ///
    /// # Panics
    /// Panics if the address does not map to a valid I/O memory location.
    pub fn write(
        &mut self,
        addr: SnesAddress,
        value: u8,
        cpu: &mut CPU,
        ppu: &mut PPU,
        apu: &mut Apu,
    ) {
        match addr.bank {
            0x00..=0x3F | 0x80..=0xBF
                if addr.addr >= IO_START_ADDRESS && addr.addr < IO_END_ADDRESS =>
            {
                match addr.addr {
                    0x2000..0x2100 => {}
                    #[cfg(not(tarpaulin_include))]
                    0x2100..0x2140 => self.write_ppu(value, addr, ppu),
                    0x2140..0x4380 => self.write_cpu(value, addr, cpu, apu),
                    0x4380..0x6000 => {}

                    _ => Self::panic_invalid_addr(addr),
                }
            }
            _ => Self::panic_invalid_addr(addr),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::snes_address::snes_addr;

    fn init_all() -> (Io, CPU, PPU, Apu) {
        let io = Io::new();
        let cpu = CPU::poweron();
        let ppu = PPU::new();
        let apu = Apu::new();

        (io, cpu, ppu, apu)
    }

    #[test]
    fn test_write_to_open_bus() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        // Write to an open bus address
        let open_bus_addr = snes_addr!(0:0x5000);
        io.write(open_bus_addr, 0xAB, &mut cpu, &mut ppu, &mut apu);
    }

    #[test]
    fn test_nmiten_register_write() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let nmiten_addr = snes_addr!(0:0x4200);
        let writen_value = 0x11;
        io.write(nmiten_addr, writen_value, &mut cpu, &mut ppu, &mut apu);

        assert_eq!(io.nmitimen, writen_value);
    }

    #[test]
    fn test_wrio_register_write() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let wrio_addr = snes_addr!(0:0x4201);
        let writen_value = 0x11;
        io.write(wrio_addr, writen_value, &mut cpu, &mut ppu, &mut apu);

        assert_eq!(io.wrio, writen_value);
    }

    #[test]
    fn test_wrmpya_wrmpyb_register_write() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let wrmpya_addr = snes_addr!(0:0x4202);
        let wrmpyb_addr = snes_addr!(0:0x4203);
        let rdmpyl_addr = snes_addr!(0:0x4216);
        let rdmpyh_addr = snes_addr!(0:0x4217);
        let value_wrmpya = 0x10;
        let value_wrmpyb = 0x25;
        io.write(wrmpya_addr, value_wrmpya, &mut cpu, &mut ppu, &mut apu);
        io.write(wrmpyb_addr, value_wrmpyb, &mut cpu, &mut ppu, &mut apu);

        assert_eq!(io.wrmpya, value_wrmpya);
        assert_eq!(io.wrmpyb, value_wrmpyb);
        assert_eq!(io.rdmpy, (io.wrmpya as u16) * (io.wrmpyb as u16));

        let rdmpyl_value = io.read(rdmpyl_addr, &mut cpu, &mut ppu, &mut apu);
        let rdmpyh_value = io.read(rdmpyh_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(rdmpyl_value, (io.rdmpy as u8));
        assert_eq!(rdmpyh_value, (io.rdmpy >> 8) as u8);
    }

    #[test]
    fn test_wrdiv_wrdivb_register_write() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let wrdivl_addr = snes_addr!(0:0x4204);
        let wrdivh_addr = snes_addr!(0:0x4205);
        let wrdivb_addr = snes_addr!(0:0x4206);
        let rddivl_addr = snes_addr!(0:0x4214);
        let rddivh_addr = snes_addr!(0:0x4215);
        let rdmpyl_addr = snes_addr!(0:0x4216);
        let rdmpyh_addr = snes_addr!(0:0x4217);
        let value_wrdivl = 0x10;
        let value_wrdivh = 0x25;
        let value_wrdiv: u16 = 0x2510;
        let value_wrdivb = 0x30;
        io.write(wrdivl_addr, value_wrdivl, &mut cpu, &mut ppu, &mut apu);
        io.write(wrdivh_addr, value_wrdivh, &mut cpu, &mut ppu, &mut apu);
        io.write(wrdivb_addr, value_wrdivb, &mut cpu, &mut ppu, &mut apu);

        assert_eq!(io.wrdivl, value_wrdivl);
        assert_eq!(io.wrdivh, value_wrdivh);
        assert_eq!(io.wrdivb, value_wrdivb);
        assert_eq!(io.rddiv, value_wrdiv / value_wrdivb as u16);
        assert_eq!(io.rdmpy, value_wrdiv % value_wrdivb as u16);

        let rdmpyl_value = io.read(rdmpyl_addr, &mut cpu, &mut ppu, &mut apu);
        let rdmpyh_value = io.read(rdmpyh_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(rdmpyl_value, (io.rdmpy as u8));
        assert_eq!(rdmpyh_value, (io.rdmpy >> 8) as u8);
        let rddivl_value = io.read(rddivl_addr, &mut cpu, &mut ppu, &mut apu);
        let rddivh_value = io.read(rddivh_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(rddivl_value, (io.rddiv as u8));
        assert_eq!(rddivh_value, (io.rddiv >> 8) as u8);
    }

    #[test]
    fn test_htimel_vtimel_register_write() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let htimel_addr = snes_addr!(0:0x4207);
        let htimeh_addr = snes_addr!(0:0x4208);
        let vtimel_addr = snes_addr!(0:0x4209);
        let vtimeh_addr = snes_addr!(0:0x420A);
        let value_htimel = 0x10;
        let value_htimeh = 0x25;
        let value_vtimel = 0x30;
        let value_vtimeh = 0x45;
        io.write(htimel_addr, value_htimel, &mut cpu, &mut ppu, &mut apu);
        io.write(htimeh_addr, value_htimeh, &mut cpu, &mut ppu, &mut apu);
        io.write(vtimel_addr, value_vtimel, &mut cpu, &mut ppu, &mut apu);
        io.write(vtimeh_addr, value_vtimeh, &mut cpu, &mut ppu, &mut apu);

        assert_eq!(io.htimel, value_htimel);
        assert_eq!(io.htimeh, value_htimeh);
        assert_eq!(io.vtimel, value_vtimel);
        assert_eq!(io.vtimeh, value_vtimeh);
    }

    #[test]
    fn test_mdmaen_register_write() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let mdmaen_addr = snes_addr!(0:0x420B);
        let value_mdmaen = 0x10;
        io.write(mdmaen_addr, value_mdmaen, &mut cpu, &mut ppu, &mut apu);

        assert_eq!(io.mdmaen, value_mdmaen);
    }

    #[test]
    fn test_hdmaen_register_write() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let hdmaen_addr = snes_addr!(0:0x420C);
        let value_hdmaen = 0x10;
        io.write(hdmaen_addr, value_hdmaen, &mut cpu, &mut ppu, &mut apu);

        assert_eq!(io.hdmaen, value_hdmaen);
    }

    #[test]
    fn test_memsel_register_write() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let memsel_addr = snes_addr!(0:0x420D);
        let value_memsel = 0x10;
        io.write(memsel_addr, value_memsel, &mut cpu, &mut ppu, &mut apu);

        assert_eq!(io.memsel, value_memsel);
    }

    #[test]
    fn test_rdnmi_register_read() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let rdnmi_addr = snes_addr!(0:0x4210);
        let value_rdnmi = 0xFF;
        io.rdnmi = value_rdnmi;

        let read_value = io.read(rdnmi_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(read_value, value_rdnmi);
        let second_read_value = io.read(rdnmi_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(second_read_value, 0b0111_1111);
    }

    #[test]
    fn test_timeup_register_read() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let timeup_addr = snes_addr!(0:0x4211);
        let value_timeup = 0xFF;
        io.timeup = value_timeup;

        let read_value = io.read(timeup_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(read_value, value_timeup);
        let second_read_value = io.read(timeup_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(second_read_value, 0b0111_1111);
    }

    #[test]
    fn test_hvbjoy_register_read() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let hvbjoy_addr = snes_addr!(0:0x4212);
        let value_hvbjoy = 0xFF;
        io.hvbjoy = value_hvbjoy;

        let read_value = io.read(hvbjoy_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(read_value, value_hvbjoy);
    }

    #[test]
    fn test_joy_autoread_result_register_read() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();

        let joy1l_addr = snes_addr!(0:0x4218);
        let joy1h_addr = snes_addr!(0:0x4219);
        let joy2l_addr = snes_addr!(0:0x421A);
        let joy2h_addr = snes_addr!(0:0x421B);
        let joy3l_addr = snes_addr!(0:0x421C);
        let joy3h_addr = snes_addr!(0:0x421D);
        let joy4l_addr = snes_addr!(0:0x421E);
        let joy4h_addr = snes_addr!(0:0x421F);
        let value_joy1: u16 = 0xF00F;
        let value_joy2: u16 = 0xE00E;
        let value_joy3: u16 = 0xD00D;
        let value_joy4: u16 = 0xC00C;
        io.joy1 = value_joy1;
        io.joy2 = value_joy2;
        io.joy3 = value_joy3;
        io.joy4 = value_joy4;

        let joy1l_value = io.read(joy1l_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(joy1l_value, value_joy1 as u8);
        let joy1h_value = io.read(joy1h_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(joy1h_value, (value_joy1 >> 8) as u8);
        let joy2l_value = io.read(joy2l_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(joy2l_value, value_joy2 as u8);
        let joy2h_value = io.read(joy2h_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(joy2h_value, (value_joy2 >> 8) as u8);
        let joy3l_value = io.read(joy3l_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(joy3l_value, value_joy3 as u8);
        let joy3h_value = io.read(joy3h_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(joy3h_value, (value_joy3 >> 8) as u8);
        let joy4l_value = io.read(joy4l_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(joy4l_value, value_joy4 as u8);
        let joy4h_value = io.read(joy4h_addr, &mut cpu, &mut ppu, &mut apu);
        assert_eq!(joy4h_value, (value_joy4 >> 8) as u8);
    }

    #[test]
    fn test_dma_registers() {
        let (mut io, mut cpu, mut ppu, mut apu) = init_all();
        let cpu_open_bus_value = 0xE4;
        cpu.data_bus = cpu_open_bus_value;

        let mut value_inc = 0;
        for channel_nb in (0..8) {
            let channel_addr = snes_addr!(0:0x4300 + 0x10 * channel_nb);

            for dma_reg in (0x0..=0xF) {
                let reg_addr = snes_addr!(0:channel_addr.addr + dma_reg);

                io.write(reg_addr, value_inc, &mut cpu, &mut ppu, &mut apu);
                let read_value = io.read(reg_addr, &mut cpu, &mut ppu, &mut apu);
                match dma_reg {
                    0x0 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].dmap, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x1 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].bbad, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x2 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].a1tl, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x3 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].a1th, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x4 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].a1b, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x5 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].dasl, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x6 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].dash, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x7 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].dasb, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x8 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].a2al, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0x9 => {
                        assert_eq!(io.dma_channels[channel_nb as usize].a2ah, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0xA => {
                        assert_eq!(io.dma_channels[channel_nb as usize].nltr, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    0xB | 0xF => {
                        assert_eq!(io.dma_channels[channel_nb as usize].unused, value_inc);
                        assert_eq!(read_value, value_inc);
                    }
                    _ => assert_eq!(read_value, cpu_open_bus_value),
                }

                value_inc += 1;
            }
        }
    }
}

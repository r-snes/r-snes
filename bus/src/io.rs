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
    nmitimen: u8,
    wrio: u8,

    wrmpya: u8,
    wrmpyb: u8,

    wrdivl: u8,
    wrdivh: u8,
    wrdivb: u8,

    htimel: u8,
    htimeh: u8,
    vtimel: u8,
    vtimeh: u8,

    mdmaen: u8,
    hdmaen: u8,
    memsel: u8,

    rddiv: u16,
    rdmpy: u16,

    rdnmi: u8,
    timeup: u8,
    hvbjoy: u8,

    joy1: u16,
    joy2: u16,
    joy3: u16,
    joy4: u16,

    dma_channels: [DMAChannel; 8],
}

#[derive(Copy, Clone)]
pub struct DMAChannel {
    dmap: u8,

    bbad: u8,

    a1tl: u8,
    a1th: u8,
    a1b: u8,

    dasl: u8,
    dash: u8,
    dasb: u8,

    a2al: u8,
    a2ah: u8,

    nltr: u8,

    unused: u8,
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
            0x2180 => todo!("0x2180-0x2183 : Implement Rom S-WRAM reads"),

            // JOYSER0/JOYSER1 - manual controller reading not implemented
            0x4016 => todo!("0x4016 : Implement JOYSER0 register read"),
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
            0x2180..=0x2183 => todo!("0x2180-0x2183 : Implement Rom S-WRAM writes"),

            // JOYOUT - manual controller reading not implemented
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
            0x4208 => self.htimeh = value & 1,
            // Screen timer Vertical target values
            0x4209 => self.vtimel = value,
            0x420A => self.vtimeh = value & 1,

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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use common::snes_address::snes_addr;

//     #[test]
//     fn test_good_map_addr() {
//         for bank in (0x00..=0x3F).chain(0x80..=0xBF) {
//             for addr in IO_START_ADDRESS..IO_END_ADDRESS {
//                 let address: SnesAddress = snes_addr!(bank:addr);
//                 assert_eq!(Io::to_offset(address), addr as usize);
//             }
//         }
//     }

//     #[test]
//     #[should_panic]
//     fn test_bad_map_addr_panics() {
//         Io::to_offset(snes_addr!(0:IO_START_ADDRESS - 0x0321));
//     }

//     #[test]
//     #[should_panic]
//     fn test_bad_map_addr_panics2() {
//         Io::to_offset(snes_addr!(0x0F:IO_END_ADDRESS + 0x34EF));
//     }

//     #[test]
//     #[should_panic(expected = "Incorrect access to the IO at address: E32345")]
//     fn test_bad_map_addr_panic_message_read() {
//         let io = Io::new();

//         io.read(snes_addr!(0xE3:0x2345));
//     }

//     #[test]
//     #[should_panic(expected = "Incorrect access to the IO at address: E32345")]
//     fn test_bad_map_addr_panic_message_write() {
//         let mut io = Io::new();

//         io.write(snes_addr!(0xE3:0x2345), 0x43);
//     }

//     #[test]
//     fn test_simple_read_write() {
//         let mut wram = Io::new();
//         let first_addr = snes_addr!(0:IO_START_ADDRESS);
//         let second_addr = snes_addr!(0x9F:IO_START_ADDRESS);

//         wram.write(first_addr, 0x43);
//         assert_eq!(wram.read(first_addr), 0x43);

//         wram.write(second_addr, 0x43);
//         assert_eq!(wram.read(second_addr), 0x43);
//     }
// }

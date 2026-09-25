use std::ops::{Deref, DerefMut};

use super::*;
use bus::cartridge::{Cartridge, test_rom::*};
use common::snes_addr;
use common::snes_address::SnesAddress;
use common::u16_split::U16Split;
use cpu::registers::RegisterP;
use duplicate::duplicate_item;
use ppu::constants::{MASTER_CYCLES_PER_SCANLINE, SCANLINES_PER_FRAME};

pub fn make_rsnes() -> RSnesCore {
    let rom_data = create_valid_lorom(0x20000);
    let (rom_path, _dir) = create_temp_rom(&rom_data);
    RSnesCore::load_rom(&rom_path).unwrap()
}

pub struct TestRsnesCore(pub RSnesCore);

impl AsRef<RSnesCore> for TestRsnesCore {
    fn as_ref(&self) -> &RSnesCore {
        &self.0
    }
}
impl Deref for TestRsnesCore {
    type Target = RSnesCore;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AsMut<RSnesCore> for TestRsnesCore {
    fn as_mut(&mut self) -> &mut RSnesCore {
        &mut self.0
    }
}
impl DerefMut for TestRsnesCore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl TestRsnesCore {
    const NMI_MARKER_ADDR: SnesAddress = snes_addr!(0:0x1FFE);
    const IRQ_MARKER_ADDR: SnesAddress = snes_addr!(0:0x1FFF);

    pub fn new() -> Self {
        let mut core = make_rsnes();
        core.bus.wram.write(Self::NMI_MARKER_ADDR, 0x42);
        core.bus.wram.write(Self::IRQ_MARKER_ADDR, 0x42);

        // init code is a BRA (branch always) that branches to itself:
        // we keep the CPU looping on one instruction by doing this,
        // avoiding executing uninitialised memory
        let init_code: [u8; _] = [
            0x58,        // CLI opcode: clear the "disable IRQ" flag, so we can see IRQs
            0x80,        // BRA opcode
            -2_i8 as u8, // jump 2 bytes backwards: BRA is 2 bytes long, so we loop
        ];
        // write a 0x99 at the NMI marker
        // to be sure we only write one byte, we have to put the accumulator in 8-bit mode first
        let interrupt_handler = |marker_addr: SnesAddress| {
            [
                0xe2, // SEP opcode used to set CPU flags
                RegisterP {
                    M: true,
                    ..0.into()
                }
                .into(), // set the M flag for 8-bit memory
                0xa9, // LDA imm opcode, to load a byte in A
                0x99, // the byte we're going to write
                0x8f, // STA absl opcode, to store A somewhere
                *marker_addr.addr.lo(),
                *marker_addr.addr.hi(),
                marker_addr.bank,
                0x40, // RTI opcode: return from interrupt
            ]
        };

        for (interrupt_vec, routine_addr, interrupt_code) in [
            (0xFFFC, 1000_u16, &init_code as &[u8]), // reset
            (0xFFEA, 1100_u16, &interrupt_handler(Self::NMI_MARKER_ADDR)), // nmi native
            (0xFFFA, 1100_u16, &interrupt_handler(Self::NMI_MARKER_ADDR)), // nmi emu
            (0xFFEE, 1200_u16, &interrupt_handler(Self::IRQ_MARKER_ADDR)), // irq native
            (0xFFFE, 1200_u16, &interrupt_handler(Self::IRQ_MARKER_ADDR)), // irq emu
        ] {
            let int_vec_addr = Cartridge::get_lorom_offset(snes_addr!(0:interrupt_vec)).unwrap();
            core.bus.cart.rom[int_vec_addr] = *routine_addr.lo();
            core.bus.cart.rom[int_vec_addr + 1] = *routine_addr.hi();
            core.bus.wram.data[routine_addr as usize..routine_addr as usize + interrupt_code.len()]
                .copy_from_slice(interrupt_code);
        }

        Self(core)
    }

    #[duplicate_item(
            DUP_name            DUP_addr;
            [has_nmi_occured]   [Self::NMI_MARKER_ADDR];
            [has_irq_occured]   [Self::IRQ_MARKER_ADDR];
        )]
    pub fn DUP_name(&mut self) -> bool {
        // let the CPU complete an interrupt routine in case one was just requested
        for _ in 0..5000 {
            self.0.update_cpu_cycles();
        }
        self.0.bus.wram.read(DUP_addr).0 == 0x99
    }
}

/// Ticks the core emulator for a given number of master cycles
pub fn tick_core(rsnes: &mut RSnesCore, cycles: u64) {
    for _ in 0..cycles {
        rsnes.update();
    }
}

/// Ticks until the PPU sits at the very start of `target`.
pub fn advance_core_to_scanline(rsnes: &mut RSnesCore, target: u16) {
    let cap = (SCANLINES_PER_FRAME as u32 + 1) * MASTER_CYCLES_PER_SCANLINE;
    for _ in 0..cap {
        rsnes.update();
        if rsnes.ppu.scanline == target && rsnes.ppu.h_cycles == 0 {
            return;
        }
    }
    panic!("never reached the start of scanline {target}");
}

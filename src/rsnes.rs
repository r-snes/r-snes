#[cfg(feature = "plugins")]
mod rsnes_plugin;

use bus::io::IrqMode;
#[cfg(feature = "plugins")]
use plugins::plugin::Plugin;
use ppu::ppu::PpuEvent;
use ppu::ppu::ScanlineKind;
use std::ops::DerefMut;
#[cfg(feature = "plugins")]
use std::{cell::RefCell, rc::Rc};

use apu::Apu;
use bus::Bus;
use common::snes_address::SnesAddress;
use cpu::cpu::CPU;
use cpu::cpu::CycleResult;

use bus::rom::header::RomHeader;
use ppu::ppu::PPU;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

/// R-SNES core: struct containing all the emulated hardware components,
/// without anything else: no GUI handles, no additional resources for
/// plugin execution; just the hardware components.
pub struct RSnesCore {
    pub _rom_path: PathBuf,
    pub bus: Bus,
    pub cpu: CPU,
    pub ppu: PPU,
    pub ppu_renderer: ppu::Renderer,
    pub apu: Apu,
    pub master_cycles: u64,
    pub cpu_master_cycles_to_wait: u32,
    pub apu_cycle_debt: u64,
}

/// Snapshot of the loaded ROM's metadata for display in the GUI.
///
/// Clones the header rather than borrowing so the GUI can hold it across
/// frames without keeping the core borrowed.
#[derive(Clone)]
pub struct RomInfo {
    pub path: PathBuf,
    /// Actual size of the ROM file on disk, in KB — not the header's
    /// `rom_size` exponent, which is what the cartridge *claims*.
    pub file_size_kb: usize,
    pub header: RomHeader,
}
impl RSnesCore {
    /// Builds a display snapshot of the loaded ROM's metadata.
    pub fn rom_info(&self) -> RomInfo {
        RomInfo {
            path: self._rom_path.clone(),
            file_size_kb: self.bus.rom.data.len() / 1024,
            header: self.bus.rom.header.clone(),
        }
    }
}
/// R-SNES core + optionally lua runtime for plugin execution (in
/// case the feature is enabled)
pub struct RSnesEmu {
    #[cfg(not(feature = "plugins"))]
    core: RSnesCore,

    #[cfg(feature = "plugins")]
    core: Rc<RefCell<RSnesCore>>,

    #[cfg(feature = "plugins")]
    plugin: Option<Plugin>,
}

impl RSnesCore {
    pub const MASTER_CLOCK_HZ: u64 = 21_477_300;

    pub fn load_rom<P: AsRef<Path>>(rom_path: &P) -> Result<Self, Box<dyn Error>> {
        let bus = Bus::new(rom_path)?;
        let cpu = CPU::poweron();
        let ppu = PPU::new();
        let ppu_renderer = ppu::Renderer::new();
        let apu = Apu::new();

        Ok(Self {
            _rom_path: rom_path.as_ref().to_path_buf().clone(),
            bus,
            cpu,
            ppu,
            ppu_renderer,
            apu,
            master_cycles: 0,
            cpu_master_cycles_to_wait: 0,
            apu_cycle_debt: 0,
        })
    }

    fn dma_transfer(&mut self) {
        let mdmaen = self.bus.io.mdmaen;

        for channel_nb in 0..8 {
            if mdmaen & (1 << channel_nb) == 0 {
                continue;
            }
            self.execute_dma_channel(channel_nb);
        }

        self.bus.io.mdmaen = 0;
    }

    fn execute_dma_channel(&mut self, channel_nb: u8) {
        let ch = &self.bus.io.dma_channels[channel_nb as usize];

        // Get transfer parameters from channel DMAP register
        let direction = (ch.dmap >> 7) & 1;
        let fixed = (ch.dmap >> 3) & 1;
        let decrement = (ch.dmap >> 4) & 1;
        let mode = ch.dmap & 0x07;
        let ch_b_addr = ch.bbad;

        let mut a_addr = ch.a1t;

        // 0x0000 means 65536 bytes, u32 needed to not overflow
        let remaining: u32 = {
            let raw = ch.das;
            if raw == 0 { 0x10000 } else { raw as u32 }
        };

        let b_offsets: &[u8] = match mode {
            0 => &[0],
            1 => &[0, 1],
            2 | 6 => &[0, 0],
            3 | 7 => &[0, 0, 1, 1],
            4 => &[0, 1, 2, 3],
            5 => &[0, 1, 0, 1],
            _ => unreachable!(),
        };

        for pattern_idx in 0..remaining {
            let b_offset = b_offsets[pattern_idx as usize % b_offsets.len()];
            let b_addr = SnesAddress {
                bank: 0x00,
                addr: 0x2100 | (ch_b_addr as u16 + b_offset as u16),
            };

            let (src, dest) = if direction == 0 {
                (a_addr, b_addr)
            } else {
                (b_addr, a_addr)
            };
            let byte = self.bus.read(src, &mut self.ppu, &mut self.apu);
            self.bus.write(dest, byte, &mut self.ppu, &mut self.apu);

            if fixed == 0 {
                if decrement == 0 {
                    a_addr.increment();
                } else {
                    a_addr.decrement();
                }
            }

            // Each byte transferred takes 8 master cycles - ROUGH WAY TO HANDLE IT, TO CHANGE LATER
            self.cpu_master_cycles_to_wait += 8;
        }

        // Reset DMA channel registers
        let ch = &mut self.bus.io.dma_channels[channel_nb as usize];
        ch.das = 0;
        ch.a1t.addr = a_addr.addr;
    }

    /// This function will be called every master cycle, it will either decrease the
    /// number of master cycles to wait or execute a cpu cycle
    fn update_cpu_cycles(&mut self) {
        if self.cpu_master_cycles_to_wait > 0 {
            self.cpu_master_cycles_to_wait -= 1;
            return;
        }

        // Check for DMA start
        if self.bus.io.mdmaen != 0 {
            self.dma_transfer();
        }

        match self.cpu.cycle() {
            CycleResult::Internal => {
                self.cpu_master_cycles_to_wait = 6; // TODO : Confirm internal cpu cycle is 6 master cycles
            }
            CycleResult::Read => {
                let addr = *self.cpu.addr_bus();
                let byte = self.bus.read(addr, &mut self.ppu, &mut self.apu);

                self.cpu.data_bus = byte;

                // Default to 6 cycles for now
                self.cpu_master_cycles_to_wait = 6; // TODO : have the bus return the number of cycle to wait
            }
            CycleResult::Write => {
                let addr = *self.cpu.addr_bus();
                let byte = self.cpu.data_bus;

                self.bus.write(addr, byte, &mut self.ppu, &mut self.apu);

                // Default to 6 cycles for now
                self.cpu_master_cycles_to_wait = 6; // TODO : have the bus return the number of cycle to wait
            }
        }
    }

    /// Advance the APU by however many of its own 1.024 MHz cycles are
    /// owed, given one more master clock cycle has just elapsed.
    /// ~1 APU cycle per 20.97 master cycles
    fn update_apu_cycles(&mut self) {
        self.apu_cycle_debt += Apu::CLOCK_HZ;
        while self.apu_cycle_debt >= Self::MASTER_CLOCK_HZ {
            self.apu_cycle_debt -= Self::MASTER_CLOCK_HZ;
            self.apu.step(1);
        }
    }

    /// This function will be called every master cycle, it will update the CPU, PPU and APU state accordingly
    pub fn update(&mut self) {
        self.update_cpu_cycles();
        self.update_apu_cycles();
        self.update_ppu_cycles();

        self.master_cycles += 1;
    }

    fn update_ppu_cycles(&mut self) {
        match self.ppu.tick() {
            PpuEvent::None => return,
            PpuEvent::DotStart => {}
            PpuEvent::HBlankStart => self.on_hblank_start(),
            PpuEvent::ScanlineStart(kind) => {
                self.bus.io.set_hblank(false); // H-Blank ends
                match kind {
                    ScanlineKind::Normal => {}
                    ScanlineKind::VBlankStart => self.on_vblank_start(),
                    ScanlineKind::FrameStart => self.on_frame_start(),
                }
            }
        }

        // Everything below is implied by "a new dot began", which every
        // variant except `None` guarantees.
        self.check_hv_irq();
        // TODO : joypad auto read check
    }

    /// Start of H-Blank (dot 274) on the current scanline.
    fn on_hblank_start(&mut self) {
        self.bus.io.set_hblank(true);

        if let Some(y) = self.ppu.visible_line() {
            self.ppu_renderer.render_scanline(&self.ppu, y);
        }

        // HDMA transfers run in the H-Blank of scanlines 0 through the last
        // visible one, never during V-Blank.
        if self.ppu.scanline < self.ppu.vblank_start_line() {
            if self.bus.io.hdmaen != 0 {
                todo!(
                    "HDMA transfer on scanline {}: channels {:08b} enabled via HDMAEN",
                    self.ppu.scanline,
                    self.bus.io.hdmaen
                );
            }
        }
    }

    /// First scanline of V-Blank (225, or 240 when SETINI's overscan bit is set).
    fn on_vblank_start(&mut self) {
        self.bus.io.set_vblank(true);
        self.bus.io.set_nmi_flag(true);

        // Hardware reloads the internal OAM address from OAMADD here,
        // but only when the screen isn't being force-blanked.
        if !self.ppu.force_blank() {
            // TODO : Reload OAM address
        }

        if self.bus.io.nmi_enabled() {
            self.cpu.nmi();
        }
    }

    /// Scanline 0: V-Blank ends and a new frame begins. Scanline 0 is the
    /// pre-render line, nothing is drawn on it, the first visible line is 1.
    fn on_frame_start(&mut self) {
        self.bus.io.set_vblank(false);
        self.bus.io.set_nmi_flag(false);

        // The last visible scanline was rendered back at line 224's
        // H-Blank, so the back buffer is complete and safe to publish.
        self.ppu_renderer.swap_buffers();

        // HDMA init for the new frame.
        if self.bus.io.hdmaen != 0 {
            todo!(
                "HDMA init at frame start: channels {:08b} enabled via HDMAEN",
                self.bus.io.hdmaen
            );
        }
    }

    /// NMITIMEN bits 5-4 select the H/V timer mode.
    fn check_hv_irq(&mut self) {
        let (h, v) = (self.ppu.dot(), self.ppu.scanline);

        let hit = match self.bus.io.irq_mode() {
            IrqMode::Disabled => return,
            IrqMode::H => h == self.bus.io.htime,
            IrqMode::V => v == self.bus.io.vtime && h == 0,
            IrqMode::HV => v == self.bus.io.vtime && h == self.bus.io.htime,
        };

        if hit {
            self.bus.io.set_timer_flag(true);
            self.cpu.irq();
        }
    }

    /// Checks if the CPU is about to execute the first cycle of an instruction,
    /// and if so, also return the opcode that is about to be read by the CPU
    pub fn is_cpu_instr_start(&mut self) -> Option<u8> {
        if self.cpu_master_cycles_to_wait != 0 || !self.cpu.is_instr_start() {
            None
        } else {
            let opcode = self
                .bus
                .read(*self.cpu.addr_bus(), &mut self.ppu, &mut self.apu);
            Some(opcode)
        }
    }
}

impl RSnesEmu {
    #[cfg_attr(
        feature = "plugins",
        expect(unused, reason = "unused for now, but makes sense to have")
    )]
    pub fn new(core: RSnesCore) -> Self {
        cfg_select! {
            feature = "plugins" => Self {
                core: Rc::new(RefCell::new(core)),
                plugin: None,
            },
            _ => Self { core },
        }
    }

    #[cfg(feature = "plugins")]
    pub fn new_with_plugin(
        core: RSnesCore,
        mut plugin: Option<Plugin>,
    ) -> Result<Self, piccolo::ExternError> {
        let rc = Rc::new(RefCell::new(core));

        if let Some(plugin) = &mut plugin {
            RSnesCore::inject_into_lua(&rc, plugin);
            plugin.run_init()?;
        }
        Ok(Self { core: rc, plugin })
    }

    pub fn core_mut(&mut self) -> impl DerefMut<Target = RSnesCore> {
        #[cfg(feature = "plugins")]
        return self.core.borrow_mut();

        #[cfg(not(feature = "plugins"))]
        return &mut self.core;
    }

    #[cfg(feature = "plugins")]
    pub fn plugin_mut(&mut self) -> Option<&mut Plugin> {
        self.plugin.as_mut()
    }

    #[cfg(not(feature = "plugins"))]
    pub fn update(&mut self) {
        self.core.update();
    }

    #[cfg(feature = "plugins")]
    pub fn update(&mut self) -> Result<(), piccolo::ExternError> {
        let mut rsnes_mut = self.core.borrow_mut();

        rsnes_mut.update();

        if let Some(plugin) = self.plugin.as_mut()
            && let Some(opcode) = rsnes_mut.is_cpu_instr_start()
        {
            let addr = *rsnes_mut.cpu.addr_bus();
            drop(rsnes_mut);
            return plugin.run_on_instr(opcode, addr);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus::rom::test_rom::*;
    use common::snes_addr;

    pub(super) fn make_rsnes() -> RSnesCore {
        let rom_data = create_valid_lorom(0x20000);
        let (rom_path, _dir) = create_temp_rom(&rom_data);
        RSnesCore::load_rom(&rom_path).unwrap()
    }

    fn set_dma_channel(
        rsnes: &mut RSnesCore,
        channel: usize,
        dmap: u8,
        src_bank: u8,
        src_addr: u16,
        size: u16,
    ) {
        let ch = &mut rsnes.bus.io.dma_channels[channel];
        ch.dmap = dmap;
        ch.bbad = 0xFF; // 0x21FF: safe no-op destination because useful memory zones not implemented yet
        ch.a1t.bank = src_bank;
        ch.a1t.addr = src_addr;
        ch.das = size;
    }

    #[test]
    fn test_mdmaen_cleared_after_transfer() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.mdmaen = 0b0000_0001;
        set_dma_channel(&mut rsnes, 0, 0x00, 0x7E, 0x0000, 1);

        rsnes.dma_transfer();

        assert_eq!(
            rsnes.bus.io.mdmaen, 0,
            "mdmaen should be cleared after transfer"
        );
    }

    #[test]
    fn test_only_enabled_channels_run() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.mdmaen = 0b0000_0010;

        set_dma_channel(&mut rsnes, 0, 0x00, 0x7E, 0x0000, 1);
        set_dma_channel(&mut rsnes, 1, 0x00, 0x7E, 0x0000, 1);

        rsnes.dma_transfer();

        // Channel 0 was not enabled, its source address should not have changed
        let ch0 = &rsnes.bus.io.dma_channels[0];
        let ch0_addr = ch0.a1t.addr;
        assert_eq!(ch0_addr, 0x0000, "Channel 0 should not have run");
        assert_eq!(rsnes.bus.io.mdmaen, 0);
    }

    #[test]
    fn test_multiple_channels_run() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.mdmaen = 0b0000_0011;

        set_dma_channel(&mut rsnes, 0, 0x00, 0x7E, 0x0000, 2);
        set_dma_channel(&mut rsnes, 1, 0x00, 0x7E, 0x0100, 3);

        rsnes.dma_transfer();

        let ch0 = &rsnes.bus.io.dma_channels[0];
        let ch0_addr = ch0.a1t.addr;
        assert_eq!(ch0_addr, 0x0002, "Channel 0 should have advanced by 2");

        let ch1 = &rsnes.bus.io.dma_channels[1];
        let ch1_addr = ch1.a1t.addr;
        assert_eq!(ch1_addr, 0x0103, "Channel 1 should have advanced by 3");
    }

    #[test]
    fn test_a1t_increments_after_transfer() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.mdmaen = 0b0000_0001;
        set_dma_channel(&mut rsnes, 0, 0x00, 0x7E, 0x0010, 4);

        rsnes.dma_transfer();

        let ch = &rsnes.bus.io.dma_channels[0];
        let final_addr = ch.a1t.addr;
        assert_eq!(
            final_addr, 0x0014,
            "Source address should have advanced by 4"
        );
    }

    #[test]
    fn test_a1t_decrements_after_transfer() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.mdmaen = 0b0000_0001;
        set_dma_channel(&mut rsnes, 0, 0b0001_0000, 0x7E, 0x0010, 4);

        rsnes.dma_transfer();

        let ch = &rsnes.bus.io.dma_channels[0];
        let final_addr = ch.a1t.addr;
        assert_eq!(
            final_addr, 0x000C,
            "Source address should have decreased by 4"
        );
    }

    #[test]
    fn test_a1t_unchanged_in_fixed_mode() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.mdmaen = 0b0000_0001;
        set_dma_channel(&mut rsnes, 0, 0b0000_1000, 0x7E, 0x0010, 4);

        rsnes.dma_transfer();

        let ch = &rsnes.bus.io.dma_channels[0];
        let final_addr = ch.a1t.addr;
        assert_eq!(
            final_addr, 0x0010,
            "Source address should not change in fixed mode"
        );
    }

    #[test]
    fn test_das_zeroed_after_transfer() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.mdmaen = 0b0000_0001;
        set_dma_channel(&mut rsnes, 0, 0x00, 0x7E, 0x0000, 8);

        rsnes.dma_transfer();

        let ch = &rsnes.bus.io.dma_channels[0];
        assert_eq!(ch.das, 0, "das should be 0 after transfer");
    }

    /// This test isn't really relevant for now because the destination
    /// does not really registers the written value from a to b
    #[test]
    fn test_wram_source_bytes_are_read() {
        let mut rsnes = make_rsnes();

        rsnes.bus.wram.data[0x0100] = 0xAB;
        rsnes.bus.wram.data[0x0101] = 0xCD;
        rsnes.bus.wram.data[0x0102] = 0xEF;

        rsnes.bus.io.mdmaen = 0b0000_0001;
        set_dma_channel(&mut rsnes, 0, 0x00, 0x7E, 0x0100, 3);

        rsnes.dma_transfer();

        let ch = &rsnes.bus.io.dma_channels[0];
        let final_addr = ch.a1t.addr;
        assert_eq!(final_addr, 0x0103);
    }

    #[test]
    fn test_direction_b_to_a_writes_into_wram() {
        let mut rsnes = make_rsnes();

        // Pre-fill so we can confirm it changed
        rsnes.bus.wram.data[0x0200] = 0xFF;
        rsnes.bus.wram.data[0x0201] = 0xFF;
        rsnes.bus.wram.data[0x0202] = 0xFF;
        rsnes.bus.io.mdmaen = 0b0000_0001;
        set_dma_channel(&mut rsnes, 0, 0b1000_0000, 0x7E, 0x0200, 3);

        rsnes.dma_transfer();

        assert_eq!(
            &rsnes.bus.wram.data[0x0200..=0x0202],
            &[0x00, 0x00, 0x00],
            "WRAM should have been overwritten with open bus value 0x00"
        );
    }

    #[test]
    fn test_cpu_update_function() {
        let mut rsnes = make_rsnes();

        let reset_addr = bus::rom::Rom::get_lorom_offset(snes_addr!(0:0xFFFC));
        rsnes.bus.rom.data[reset_addr] = 0x00;
        rsnes.bus.rom.data[reset_addr + 1] = 0x80;

        rsnes.bus.rom.data[0] = 0xEA;
        rsnes.bus.rom.data[1] = 0xA9;
        rsnes.bus.rom.data[2] = 0x42;
        rsnes.bus.rom.data[3] = 0x8D;
        rsnes.bus.rom.data[4] = 0x34;
        rsnes.bus.rom.data[5] = 0x12;

        rsnes.update();
        assert_eq!(rsnes.cpu_master_cycles_to_wait, 6);
        rsnes.cpu_master_cycles_to_wait = 0;
        rsnes.update();
        assert_eq!(rsnes.cpu.regs().PC, 0);
        rsnes.cpu_master_cycles_to_wait = 0;

        // NO-OP
        rsnes.update();
        rsnes.cpu_master_cycles_to_wait = 0;
        assert_eq!(rsnes.cpu.regs().PC, 0x8000);
        rsnes.update();
        rsnes.cpu_master_cycles_to_wait = 0;

        // LDA
        assert_ne!(rsnes.cpu.regs().A, 0x42);
        rsnes.update();
        rsnes.cpu_master_cycles_to_wait = 0;
        rsnes.update();
        rsnes.cpu_master_cycles_to_wait = 0;

        // STA
        assert_ne!(rsnes.cpu.data_bus, 0x8D);
        rsnes.update();
        rsnes.cpu_master_cycles_to_wait = 0;
        assert_eq!(rsnes.cpu.data_bus, 0x8D);
        rsnes.update();
        rsnes.cpu_master_cycles_to_wait = 0;
        rsnes.update();
        rsnes.cpu_master_cycles_to_wait = 0;
        rsnes.update();
        rsnes.cpu_master_cycles_to_wait = 0;

        assert_eq!(rsnes.bus.wram.read(snes_addr!(0:0x1234)), 0x42);
    }
}

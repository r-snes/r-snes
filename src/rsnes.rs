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

use crate::dma::*;

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
    pub nmi_pending: bool,
    pub irq_pending: bool,
    pub apu_cycle_debt: u64,
    pub dma: Dma,
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
            nmi_pending: false,
            irq_pending: false,
            dma: Dma::default(),
        })
    }

    /// This function will be called every master cycle, it will update the
    /// CPU, PPU, APU and DMA state accordingly.
    pub fn update(&mut self) {
        self.update_ppu_cycles();
        self.update_apu_cycles();

        // DMA holds the bus while it is active, so the CPU does not run during that time.
        if !self.update_dma_cycles() {
            self.update_cpu_cycles();
        }

        self.master_cycles += 1;
    }

    /// This function will be called every master cycle, it will either decrease the
    /// number of master cycles to wait or execute a cpu cycle
    fn update_cpu_cycles(&mut self) {
        if self.cpu_master_cycles_to_wait > 0 {
            self.cpu_master_cycles_to_wait -= 1;
            return;
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

    /// Advance the DMA unit one master cycle.
    ///
    /// Returns `true` while DMA holds the bus, which is the signal for the
    /// CPU to sit out this cycle.
    fn update_dma_cycles(&mut self) -> bool {
        self.start_pending_dma();

        // HDMA preempts a running DMA transfer, but only on a
        // unit boundary, never mid-byte. The transfer's progress is left
        // untouched and resumes once the HDMA pass drains.
        if self.dma.hdma_pending && self.dma.wait == 0 {
            self.dma.hdma_pending = false;
            self.dma.hdma_queue = self.bus.io.hdmaen;
            self.dma.state = DmaState::Hdma;
            self.dma.wait = HDMA_LINE_COST - 1;
            return true;
        }

        if self.dma.state == DmaState::Idle {
            return false;
        }

        if self.dma.wait > 0 {
            self.dma.wait -= 1;
            return true;
        }

        match self.dma.state {
            DmaState::Idle => unreachable!("checked above"),
            DmaState::Startup => {
                self.dma.state = DmaState::Dma;
            }
            DmaState::Dma => self.dma_step(),
            DmaState::Hdma => self.hdma_step(),
        }

        true
    }

    /// Pick up a DMA transfer requested via MDMAEN.
    fn start_pending_dma(&mut self) {
        if self.dma.dma_running || self.bus.io.mdmaen == 0 {
            return;
        }

        // A channel enabled for both loses its general-purpose transfer;
        // HDMA wins outright and the transfer is aborted, not deferred.
        self.bus.io.mdmaen &= !self.bus.io.hdmaen;
        if self.bus.io.mdmaen == 0 {
            return;
        }

        self.dma.dma_running = true;
        self.dma.state = DmaState::Startup;

        // Sync to the next 8-cycle slot, then pay the startup overhead.
        let align = 8 - (self.master_cycles % 8) as u32;
        self.dma.wait = align + DMA_START_COST - 1;
    }

    /// One DMA action: either select the next channel, or move one byte.
    fn dma_step(&mut self) {
        let Some(progress) = self.dma.dma else {
            match self.bus.io.mdmaen {
                // Every queued channel has completed.
                0 => {
                    self.dma.dma_running = false;
                    self.dma.state = DmaState::Idle;
                    // Sync back to a whole CPU clock since the pause
                    // before the S-CPU resumes: 2-8 master cycles.
                    self.dma.wait = 8 - (self.master_cycles % 8).max(2) as u32;
                }
                // Channels run lowest bit first, one fully at a time.
                mask => {
                    let channel = mask.trailing_zeros() as u8;
                    self.dma.dma = Some(DmaProgress {
                        channel,
                        unit_index: 0,
                    });
                    self.dma.wait = DMA_CHANNEL_COST - 1;
                }
            }
            return;
        };

        self.dma_transfer_byte(progress.channel, progress.unit_index);

        let ch = &mut self.bus.io.dma_channels[progress.channel as usize];

        // DAS is a live down-counter, so a starting value of 0 naturally
        // yields 65 536 bytes: it wraps to 0xFFFF on the first decrement
        // and only reaches 0 after a full lap.
        ch.das = ch.das.wrapping_sub(1);

        if ch.das == 0 {
            self.bus.io.mdmaen &= !(1 << progress.channel);
            self.dma.dma = None;
        } else {
            self.dma.dma = Some(DmaProgress {
                unit_index: progress.unit_index.wrapping_add(1),
                channel: progress.channel,
            });
        }

        self.dma.wait = BYTE_COST - 1;
    }

    fn dma_transfer_byte(&mut self, channel: u8, unit_index: u32) {
        let ch = &self.bus.io.dma_channels[channel as usize];
        let dmap = ch.dmap;
        let a_addr = ch.a1t;

        let b_to_a = dmap & 0x80 != 0;
        // Bit 3 (fixed) takes priority over bit 4 (decrement).
        let fixed = dmap & 0x08 != 0;
        let decrement = dmap & 0x10 != 0;

        let pattern = Dma::transfer_pattern(dmap);
        let b_addr = Dma::b_address(ch.bbad, pattern[unit_index as usize % pattern.len()]);

        let (src, dst) = if b_to_a {
            (b_addr, a_addr)
        } else {
            (a_addr, b_addr)
        };
        let byte = self.bus.read(src, &mut self.ppu, &mut self.apu);
        self.bus.write(dst, byte, &mut self.ppu, &mut self.apu);

        // The A-bus address wraps inside its bank; A1B never increments.
        if !fixed {
            let ch = &mut self.bus.io.dma_channels[channel as usize];
            ch.a1t.addr = if decrement {
                ch.a1t.addr.wrapping_sub(1)
            } else {
                ch.a1t.addr.wrapping_add(1)
            };
        }
    }

    /// One HDMA action: service the next queued channel, or hand the bus back.
    fn hdma_step(&mut self) {
        if self.dma.hdma_queue == 0 {
            self.dma.hdma_init = false;

            // A preempted general-purpose transfer picks up exactly where
            // it paused.
            if self.dma.dma_running {
                self.dma.state = DmaState::Dma;
                self.dma.wait = DMA_RESUME_COST - 1;
            } else {
                self.dma.state = DmaState::Idle;
            }
            return;
        }

        let channel = self.dma.hdma_queue.trailing_zeros() as u8;
        self.dma.hdma_queue &= !(1 << channel);

        if self.dma.hdma_init {
            self.hdma_init_channel(channel);
        } else {
            self.hdma_line_channel(channel);
        }
    }

    /// Once per frame: rewind the table pointer and load the first entry.
    fn hdma_init_channel(&mut self, channel: u8) {
        self.dma.channels[channel as usize] = HdmaChannelState::default();

        let ch = &mut self.bus.io.dma_channels[channel as usize];
        ch.a2a = ch.a1t.addr;
        let indirect = ch.dmap & 0x40 != 0;

        let counter = self.hdma_read_table(channel);
        let state = &mut self.dma.channels[channel as usize];

        if counter == 0 {
            state.finished = true;
            self.dma.wait = HDMA_RELOAD_COST - 1;
            return;
        }

        state.line_counter = counter;
        state.do_transfer = true;

        let cost = if indirect {
            let lo = self.hdma_read_table(channel);
            let hi = self.hdma_read_table(channel);
            self.bus.io.dma_channels[channel as usize].das = u16::from_le_bytes([lo, hi]);
            HDMA_RELOAD_INDIRECT_COST
        } else {
            HDMA_RELOAD_COST
        };

        self.dma.wait = cost - 1;
    }

    /// Per scanline: reload the entry if the counter ran out, transfer if
    /// this line calls for it, then step the counter.
    fn hdma_line_channel(&mut self, channel: u8) {
        if self.dma.channels[channel as usize].finished {
            return;
        }

        let indirect = self.bus.io.dma_channels[channel as usize].dmap & 0x40 != 0;
        let mut cost = HDMA_CHANNEL_COST;

        if self.dma.channels[channel as usize].line_counter & 0x7F == 0 {
            let counter = self.hdma_read_table(channel);

            // A line count of 0 terminates the channel for the rest of
            // the frame — it does not mean 256.
            if counter == 0 {
                self.dma.channels[channel as usize].finished = true;
                self.dma.wait = HDMA_RELOAD_COST - 1;
                return;
            }

            let state = &mut self.dma.channels[channel as usize];
            state.line_counter = counter;
            state.do_transfer = true;

            if indirect {
                let lo = self.hdma_read_table(channel);
                let hi = self.hdma_read_table(channel);
                self.bus.io.dma_channels[channel as usize].das = u16::from_le_bytes([lo, hi]);
                cost += HDMA_RELOAD_INDIRECT_COST;
            } else {
                cost += HDMA_RELOAD_COST;
            }
        }

        if self.dma.channels[channel as usize].do_transfer {
            cost += self.hdma_transfer_unit(channel, indirect);
        }

        let state = &mut self.dma.channels[channel as usize];
        // Bit 7 is the repeat flag: set means "write every line for N
        // lines" (a gradient), clear means "write once, then hold for N
        // lines" (a flat band).
        let repeat = state.line_counter & 0x80 != 0;
        state.line_counter = state.line_counter.wrapping_sub(1);
        state.do_transfer = repeat || (state.line_counter & 0x7F) == 0;

        self.dma.wait = cost - 1;
    }

    /// Read one byte from the channel's HDMA table and advance A2A.
    fn hdma_read_table(&mut self, channel: u8) -> u8 {
        let ch = &mut self.bus.io.dma_channels[channel as usize];
        let addr = SnesAddress {
            bank: ch.a1t.bank,
            addr: ch.a2a,
        };
        ch.a2a = ch.a2a.wrapping_add(1);

        self.bus.read(addr, &mut self.ppu, &mut self.apu)
    }

    /// Move one full transfer unit (1-4 bytes per DMAP mode) and return
    /// its cycle cost.
    fn hdma_transfer_unit(&mut self, channel: u8, indirect: bool) -> u32 {
        let ch = &self.bus.io.dma_channels[channel as usize];
        let dmap = ch.dmap;
        let bbad = ch.bbad;
        let b_to_a = dmap & 0x80 != 0;
        let pattern = Dma::transfer_pattern(dmap);

        // Direct channels stream from the table itself; indirect ones
        // stream from DASB:DAS, which the table supplied.
        let bank = if indirect { ch.dasb } else { ch.a1t.bank };
        let mut addr = if indirect { ch.das } else { ch.a2a };

        let mut cost = 0;
        for &offset in pattern {
            let a_addr = SnesAddress { bank, addr };
            let b_addr = Dma::b_address(bbad, offset);

            let (src, dst) = if b_to_a {
                (b_addr, a_addr)
            } else {
                (a_addr, b_addr)
            };
            let byte = self.bus.read(src, &mut self.ppu, &mut self.apu);
            self.bus.write(dst, byte, &mut self.ppu, &mut self.apu);

            addr = addr.wrapping_add(1);
            cost += BYTE_COST;
        }

        let ch = &mut self.bus.io.dma_channels[channel as usize];
        if indirect {
            ch.das = addr;
        } else {
            ch.a2a = addr;
        }

        cost
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

    fn update_ppu_cycles(&mut self) {
        match self.ppu.tick() {
            PpuEvent::None => {}
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

        // HDMA runs in the H-Blank of scanlines 0 through the last
        // visible one, never during V-Blank
        if self.ppu.scanline < self.ppu.vblank_start_line() && self.bus.io.hdmaen != 0 {
            self.dma.hdma_pending = true;
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
            self.nmi_pending = true;
            todo!("V-Blank NMI: CPU should vector through $FFEA (native) / $FFFA (emulation)");
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
            self.dma.hdma_pending = true;
            self.dma.hdma_init = true;
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
            self.irq_pending = true;
            todo!("H/V timer IRQ: CPU should vector through $FFEE/$FFFE here");
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
    use ppu::constants::*;

    /// Ticks the core without letting the CPU run.
    fn tick_core(rsnes: &mut RSnesCore, cycles: u64) {
        rsnes.cpu_master_cycles_to_wait = u32::MAX;
        for _ in 0..cycles {
            rsnes.update();
        }
    }

    /// Ticks until the PPU sits at the very start of `target`.
    fn advance_core_to_scanline(rsnes: &mut RSnesCore, target: u16) {
        rsnes.cpu_master_cycles_to_wait = u32::MAX;
        let cap = (SCANLINES_PER_FRAME as u32 + 1) * MASTER_CYCLES_PER_SCANLINE;
        for _ in 0..cap {
            rsnes.update();
            if rsnes.ppu.scanline == target && rsnes.ppu.h_cycles == 0 {
                return;
            }
        }
        panic!("never reached the start of scanline {target}");
    }

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

    // ============================================================
    // update() - clock distribution
    // ============================================================

    /// The PPU now advances from the same clock as everything else: one
    /// tick per master cycle, regardless of what the CPU is doing.
    #[test]
    fn test_ppu_advances_with_master_clock() {
        let mut rsnes = make_rsnes();

        tick_core(&mut rsnes, 100);

        assert_eq!(rsnes.master_cycles, 100);
        assert_eq!(rsnes.ppu.h_cycles, 100);
        assert_eq!(rsnes.ppu.dot(), 25, "4 master cycles per dot");
    }

    /// The APU is driven by the same clock at its own 1.024 MHz rate.
    #[test]
    fn test_apu_cycle_debt_tracks_clock_ratio() {
        let mut rsnes = make_rsnes();
        let cycles = 1_000u64;

        tick_core(&mut rsnes, cycles);

        let expected = (cycles * Apu::CLOCK_HZ) % RSnesCore::MASTER_CLOCK_HZ;
        assert_eq!(rsnes.apu_cycle_debt, expected);
    }

    #[test]
    fn test_interrupt_flags_clear_at_poweron() {
        let rsnes = make_rsnes();
        assert!(!rsnes.nmi_pending);
        assert!(!rsnes.irq_pending);
    }

    // ============================================================
    // $4212 HVBJOY - H-Blank
    // ============================================================

    /// Bit 6 is purely positional: set on entry to dot 274, cleared when
    /// the next scanline begins.
    #[test]
    fn test_hblank_flag_tracks_dot_position() {
        let mut rsnes = make_rsnes();

        tick_core(&mut rsnes, HBLANK_START_DOT as u64 * 4 - 1);
        assert!(!rsnes.bus.io.in_hblank());

        tick_core(&mut rsnes, 1);
        assert!(rsnes.bus.io.in_hblank());

        advance_core_to_scanline(&mut rsnes, 1);
        assert!(!rsnes.bus.io.in_hblank());
    }

    // ============================================================
    // $4212 HVBJOY / $4210 RDNMI - V-Blank
    // ============================================================

    /// Both flags go up on scanline 225 and come back down on scanline 0.
    #[test]
    fn test_vblank_flags_set_and_cleared() {
        let mut rsnes = make_rsnes();
        assert!(!rsnes.bus.io.in_vblank());
        assert!(!rsnes.bus.io.nmi_flag());

        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        assert!(rsnes.bus.io.in_vblank());
        assert!(rsnes.bus.io.nmi_flag());

        advance_core_to_scanline(&mut rsnes, 0);
        assert!(!rsnes.bus.io.in_vblank());
        assert!(!rsnes.bus.io.nmi_flag());
    }

    /// The V-Blank flag stays up for every scanline in the interval, not
    /// just the first one — ROMs poll it in a loop.
    #[test]
    fn test_vblank_flag_held_for_whole_interval() {
        let mut rsnes = make_rsnes();

        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        for line in [230, 245, SCANLINES_PER_FRAME - 1] {
            advance_core_to_scanline(&mut rsnes, line);
            assert!(
                rsnes.bus.io.in_vblank(),
                "should still be in V-Blank at line {line}"
            );
        }
    }

    /// The last visible line is still outside V-Blank.
    #[test]
    fn test_last_visible_scanline_is_not_vblank() {
        let mut rsnes = make_rsnes();
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE - 1);
        assert!(!rsnes.bus.io.in_vblank());
    }

    /// Reading $4210 acknowledges the NMI. HVBJOY is positional and must
    /// not be disturbed by it.
    #[test]
    fn test_reading_rdnmi_acknowledges_without_clearing_vblank() {
        let mut rsnes = make_rsnes();
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);

        let value = rsnes
            .bus
            .read(snes_addr!(0:0x4210), &mut rsnes.ppu, &mut rsnes.apu);

        assert_eq!(value & 0x80, 0x80, "read returns the flag that was set");
        assert!(!rsnes.bus.io.nmi_flag(), "read acknowledges");
        assert!(rsnes.bus.io.in_vblank(), "HVBJOY is unaffected");
    }

    /// SETINI bit 2 moves V-Blank to line 240, and the core follows it.
    #[test]
    fn test_overscan_moves_vblank_start() {
        let mut rsnes = make_rsnes();
        rsnes.ppu.write(0x2133, 0x04);

        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        assert!(
            !rsnes.bus.io.in_vblank(),
            "line 225 is visible with overscan"
        );

        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE_OVERSCAN);
        assert!(rsnes.bus.io.in_vblank());
    }

    // ============================================================
    // V-Blank NMI
    // ============================================================

    /// With NMITIMEN bit 7 set, entering V-Blank must request an NMI.
    /// Becomes a `nmi_pending` assertion once the CPU can take interrupts.
    #[test]
    #[should_panic(expected = "V-Blank NMI")]
    fn test_vblank_nmi_requested_when_enabled() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.nmitimen = 0x80;
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
    }

    /// With NMI disabled, no request — but the RDNMI flag still goes up.
    /// That asymmetry is real hardware: the flag is positional, the
    /// interrupt is opt-in.
    #[test]
    fn test_vblank_flag_set_even_when_nmi_disabled() {
        let mut rsnes = make_rsnes();
        assert!(!rsnes.bus.io.nmi_enabled());

        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);

        assert!(rsnes.bus.io.nmi_flag());
        assert!(!rsnes.nmi_pending);
    }

    // ============================================================
    // check_hv_irq - NMITIMEN bits 5-4
    // ============================================================

    #[test]
    fn test_irq_mode_decoding() {
        let mut rsnes = make_rsnes();
        for (bits, expected) in [
            (0b0000_0000, IrqMode::Disabled),
            (0b0001_0000, IrqMode::H),
            (0b0010_0000, IrqMode::V),
            (0b0011_0000, IrqMode::HV),
        ] {
            rsnes.bus.io.nmitimen = bits;
            assert_eq!(rsnes.bus.io.irq_mode(), expected);
        }
    }

    /// Mode 1 fires wherever H reaches HTIME, on any scanline.
    #[test]
    #[should_panic(expected = "H/V timer IRQ")]
    fn test_h_irq_fires_at_htime() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.nmitimen = 0b0001_0000;
        rsnes.bus.io.htime = 100;

        tick_core(&mut rsnes, 100 * 4);
    }

    /// Mode 2 fires once per frame, at H = 0 of VTIME.
    #[test]
    #[should_panic(expected = "H/V timer IRQ")]
    fn test_v_irq_fires_on_target_scanline() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.nmitimen = 0b0010_0000;
        rsnes.bus.io.vtime = 42;

        advance_core_to_scanline(&mut rsnes, 100);
    }

    /// Mode 2 must ignore every other scanline.
    #[test]
    fn test_v_irq_silent_on_other_scanlines() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.nmitimen = 0b0010_0000;
        rsnes.bus.io.vtime = 200;

        advance_core_to_scanline(&mut rsnes, 100);
        assert!(!rsnes.irq_pending);
    }

    /// Mode 3 needs both coordinates: an HTIME match on the wrong scanline
    /// must not fire.
    #[test]
    fn test_hv_irq_ignores_htime_match_on_wrong_scanline() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.nmitimen = 0b0011_0000;
        rsnes.bus.io.htime = 100;
        rsnes.bus.io.vtime = 200;

        advance_core_to_scanline(&mut rsnes, 150);
        assert!(!rsnes.irq_pending);
    }

    /// Mode 0 must never fire, even when both counters match.
    #[test]
    fn test_irq_disabled_never_fires() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.htime = 100;
        rsnes.bus.io.vtime = 100;

        advance_core_to_scanline(&mut rsnes, 150);
        assert!(!rsnes.irq_pending);
        assert!(!rsnes.bus.io.timer_flag());
    }

    // ============================================================
    // Scanline rendering
    // ============================================================

    /// Scanline N is drawn at its H-Blank, into framebuffer row N-1.
    /// Force blank makes the renderer emit black, which is easy to detect
    /// against a pre-filled buffer.
    #[test]
    fn test_scanline_rendered_at_hblank_of_visible_line() {
        let mut rsnes = make_rsnes();
        rsnes.ppu.write(0x2100, 0x80);
        rsnes.ppu_renderer.framebuffer.fill(0xFF);

        advance_core_to_scanline(&mut rsnes, 1);
        assert_eq!(rsnes.ppu_renderer.framebuffer[0], 0xFF, "not drawn yet");

        tick_core(&mut rsnes, HBLANK_START_DOT as u64 * 4);

        let row0 = &rsnes.ppu_renderer.framebuffer[..SCREEN_WIDTH * 3];
        assert!(row0.iter().all(|&b| b == 0), "scanline 1 drew row 0");

        let row1 = &rsnes.ppu_renderer.framebuffer[SCREEN_WIDTH * 3..SCREEN_WIDTH * 6];
        assert!(row1.iter().all(|&b| b == 0xFF), "row 1 untouched");
    }

    /// Scanline 0 is the pre-render line and draws nothing.
    #[test]
    fn test_prerender_scanline_draws_nothing() {
        let mut rsnes = make_rsnes();
        rsnes.ppu.write(0x2100, 0x80);
        rsnes.ppu_renderer.framebuffer.fill(0xFF);

        tick_core(&mut rsnes, HBLANK_START_DOT as u64 * 4);

        assert!(rsnes.ppu_renderer.framebuffer.iter().all(|&b| b == 0xFF));
    }

    /// V-Blank scanlines draw nothing either.
    #[test]
    fn test_vblank_scanlines_draw_nothing() {
        let mut rsnes = make_rsnes();
        rsnes.ppu.write(0x2100, 0x80);

        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        rsnes.ppu_renderer.framebuffer.fill(0xFF);

        tick_core(&mut rsnes, MASTER_CYCLES_PER_SCANLINE as u64);

        assert!(rsnes.ppu_renderer.framebuffer.iter().all(|&b| b == 0xFF));
    }

    /// The back buffer becomes visible only when the frame completes.
    #[test]
    fn test_framebuffer_published_at_frame_start() {
        let mut rsnes = make_rsnes();

        // Park in V-Blank first so no further rendering overwrites the mark.
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        rsnes.ppu_renderer.framebuffer[0] = 0xAB;
        assert_ne!(rsnes.ppu_renderer.presented()[0], 0xAB, "not published yet");

        advance_core_to_scanline(&mut rsnes, 0);
        assert_eq!(rsnes.ppu_renderer.presented()[0], 0xAB);
    }

    // ============================================================
    // HDMA scheduling
    // ============================================================

    /// A transfer is requested in the H-Blank of every visible scanline
    /// while HDMAEN is non-zero.
    #[test]
    #[should_panic(expected = "HDMA transfer")]
    fn test_hdma_transfer_requested_during_visible_lines() {
        let mut rsnes = make_rsnes();
        rsnes.bus.io.hdmaen = 0b0000_0001;

        tick_core(&mut rsnes, HBLANK_START_DOT as u64 * 4);
    }

    /// HDMA never runs during V-Blank — that window belongs to the ROM.
    /// Reaching the end without panicking is the assertion.
    #[test]
    fn test_hdma_not_requested_during_vblank() {
        let mut rsnes = make_rsnes();
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        rsnes.bus.io.hdmaen = 0b0000_0001;

        tick_core(&mut rsnes, MASTER_CYCLES_PER_SCANLINE as u64);
    }

    /// Channels are re-initialised at the top of each frame.
    #[test]
    #[should_panic(expected = "HDMA init")]
    fn test_hdma_init_requested_at_frame_start() {
        let mut rsnes = make_rsnes();
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        rsnes.bus.io.hdmaen = 0b0000_0001;

        advance_core_to_scanline(&mut rsnes, 0);
    }

    /// Nothing is requested when HDMAEN is clear.
    #[test]
    fn test_no_hdma_when_disabled() {
        let mut rsnes = make_rsnes();
        advance_core_to_scanline(&mut rsnes, 10);
    }
}

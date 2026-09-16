use crate::rsnes::RSnesCore;
use common::{snes_addr, snes_address::SnesAddress};

/// Master cycles per byte moved. Constant regardless of MEMSEL —
/// FastROM does not speed up DMA.
pub const BYTE_COST: u32 = 8;
/// One-off cost when a DMA transfer begins.
pub const DMA_START_COST: u32 = 8;
/// Cost when a DMA channel begins, paid once per channel.
pub const DMA_CHANNEL_COST: u32 = 8;
/// Per-scanline cost when any HDMA channel is enabled.
pub const HDMA_LINE_COST: u32 = 18;
/// Per-channel cost when a channel does anything on a line.
pub const HDMA_CHANNEL_COST: u32 = 8;
/// Extra cost, only when a new indirect address must be loaded.
pub const HDMA_INDIRECT_LOAD_COST: u32 = 16;
/// Frame-start init, per direct channel.
pub const HDMA_INIT_DIRECT_COST: u32 = 8;
/// Frame-start init, per indirect channel.
pub const HDMA_INIT_INDIRECT_COST: u32 = 24;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DmaState {
    /// The CPU has the bus.
    #[default]
    Idle,
    /// Aligning to the 8-cycle grid before a DMA transfer.
    Startup,
    /// A DMA transfer holds the bus.
    Dma,
    /// HDMA holds the bus, possibly having preempted a DMA.
    Hdma,
}

/// Where a DMA transfer is currently.
///
/// The byte count and source address live in the channel's DAS/A1T
/// registers and are decremented in place, exactly as hardware does —
/// so a ROM reading them mid-transfer sees live values.
#[derive(Clone, Copy, Debug)]
pub struct DmaProgress {
    pub channel: u8,
    /// Position in the DMAP transfer pattern; survives preemption.
    pub unit_index: u32,
}

/// Per-channel HDMA state for the current frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct HdmaChannelState {
    /// Live NLTR: bit 7 = repeat, bits 6-0 = scanlines remaining.
    pub line_counter: u8,
    /// Whether this channel transfers on the current scanline.
    pub do_transfer: bool,
    /// Set when the table hit its `0` terminator; stays set until the
    /// next frame's init.
    pub finished: bool,
}

#[derive(Default)]
pub struct Dma {
    pub state: DmaState,
    /// Cycles still to burn before the next action.
    pub wait: u32,
    /// A DMA transfer is underway. Distinguishes "MDMAEN was
    /// just written" from "already running", since MDMAEN doubles as the
    /// queue of channels still to run.
    pub dma_running: bool,
    pub dma: Option<DmaProgress>,
    /// H-Blank has come due and HDMA is pending
    pub hdma_pending: bool,
    /// Whether the pending pass is the once-per-frame init.
    pub hdma_init: bool,
    /// Channels left to service in the current HDMA pass.
    pub hdma_queue: u8,
    pub channels: [HdmaChannelState; 8],
}

impl Dma {
    /// The B-bus offsets written per transfer unit, by DMAP bits 2-0.
    pub fn transfer_pattern(mode: u8) -> &'static [u8] {
        match mode & 0x07 {
            0 => &[0],
            1 => &[0, 1],
            2 | 6 => &[0, 0],
            3 | 7 => &[0, 0, 1, 1],
            4 => &[0, 1, 2, 3],
            _ => &[0, 1, 0, 1],
        }
    }

    /// B-bus address for a unit offset. The sum wraps inside the `$21xx`
    /// page rather than spilling into `$22xx`.
    pub fn b_address(bbad: u8, offset: u8) -> SnesAddress {
        snes_addr!(0x00:0x21:bbad.wrapping_add(offset))
    }
}

impl RSnesCore {
    /// Advance the DMA unit one master cycle.
    ///
    /// Returns `true` while DMA holds the bus, which is the signal for the
    /// CPU to sit out this cycle.
    pub fn update_dma_cycles(&mut self) -> bool {
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

        if self.dma.wait > 0 {
            self.dma.wait -= 1;
            return true;
        }

        match self.dma.state {
            DmaState::Idle => return false,
            DmaState::Startup => self.dma.state = DmaState::Dma,
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

                    // TODO: hardware waits 2-8 master cycles here to reach
                    // a whole CPU clock since the pause. Needs the CPU
                    // clock period (6/8/12 by MEMSEL and memory region) to compute.
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

        // DAS is a live down-counter, so a starting value of 0 gives
        // 65 536 bytes, it wraps to 0xFFFF on the first decrement
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

            // A stopped DMA transfer picks up exactly where it paused.
            if self.dma.dma_running {
                self.dma.state = DmaState::Dma;
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
            self.dma.wait = HDMA_INIT_DIRECT_COST - 1;
            return;
        }

        state.line_counter = counter;
        state.do_transfer = true;

        let cost = if indirect {
            let lo = self.hdma_read_table(channel);
            let hi = self.hdma_read_table(channel);
            self.bus.io.dma_channels[channel as usize].das = u16::from_le_bytes([lo, hi]);
            HDMA_INIT_INDIRECT_COST
        } else {
            HDMA_INIT_DIRECT_COST
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
            // the frame - it does not mean 256.
            if counter == 0 {
                self.dma.channels[channel as usize].finished = true;
                self.dma.wait = cost - 1;
                return;
            }

            let state = &mut self.dma.channels[channel as usize];
            state.line_counter = counter;
            state.do_transfer = true;

            if indirect {
                let lo = self.hdma_read_table(channel);
                let hi = self.hdma_read_table(channel);
                self.bus.io.dma_channels[channel as usize].das = u16::from_le_bytes([lo, hi]);
                cost += HDMA_INDIRECT_LOAD_COST;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::snes_addr;
    use common::snes_address::SnesAddress;
    use common::u16_split::U16Split;
    use ppu::constants::{HDMA_START_DOT, MASTER_CYCLES_PER_SCANLINE, VBLANK_START_LINE};

    use crate::test_utils::*;

    fn write_reg(rsnes: &mut RSnesCore, addr: u16, value: u8) {
        rsnes
            .bus
            .write(snes_addr!(0:addr), value, &mut rsnes.ppu, &mut rsnes.apu);
    }

    fn read_reg(rsnes: &mut RSnesCore, addr: u16) -> u8 {
        rsnes
            .bus
            .read(snes_addr!(0:addr), &mut rsnes.ppu, &mut rsnes.apu)
    }

    /// Configure a channel entirely through its $43xx registers, as a ROM would.
    fn configure_channel(
        rsnes: &mut RSnesCore,
        channel: u8,
        dmap: u8,
        bbad: u8,
        src: SnesAddress,
        count: u16,
    ) {
        let base = 0x4300 + channel as u16 * 0x10;
        write_reg(rsnes, base, dmap);
        write_reg(rsnes, base + 0x1, bbad);
        write_reg(rsnes, base + 0x2, *src.addr.lo());
        write_reg(rsnes, base + 0x3, *src.addr.hi());
        write_reg(rsnes, base + 0x4, src.bank);
        write_reg(rsnes, base + 0x5, *count.lo());
        write_reg(rsnes, base + 0x6, *count.hi());
    }

    fn channel_a1t(rsnes: &mut RSnesCore, channel: u8) -> SnesAddress {
        let base = 0x4300 + channel as u16 * 0x10;
        let lo = read_reg(rsnes, base + 0x2);
        let hi = read_reg(rsnes, base + 0x3);
        let bank = read_reg(rsnes, base + 0x4);
        snes_addr!(bank:hi:lo)
    }

    fn channel_das(rsnes: &mut RSnesCore, channel: u8) -> u16 {
        let base = 0x4300 + channel as u16 * 0x10;
        u16::from_le_bytes([read_reg(rsnes, base + 0x5), read_reg(rsnes, base + 0x6)])
    }

    /// Kick off a transfer via $420B and run until every queued channel has
    /// finished. Returns the master cycles consumed.
    fn run_dma(rsnes: &mut RSnesCore, channels: u8, cap: u64) -> u64 {
        let start = rsnes.master_cycles;
        write_reg(rsnes, 0x420B, channels);

        for _ in 0..cap {
            rsnes.update();
            if rsnes.bus.io.mdmaen == 0 {
                return rsnes.master_cycles - start;
            }
        }
        panic!("DMA did not complete within {cap} master cycles");
    }

    fn fill_wram(rsnes: &mut RSnesCore, at: SnesAddress, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            let bank = at.bank;
            let addr = at.addr.wrapping_add(i as u16);
            rsnes.bus.wram.write(snes_addr!(bank:addr), b);
        }
    }

    /// Point VMAIN at "increment after the $2119 write" so a mode-1 transfer
    /// builds consecutive 16-bit VRAM words.
    fn vram_word_mode(rsnes: &mut RSnesCore) {
        write_reg(rsnes, 0x2115, 0x80);
        write_reg(rsnes, 0x2116, 0x00);
        write_reg(rsnes, 0x2117, 0x00);
    }

    const HDMA_TABLE: SnesAddress = snes_addr!(0x7E:0x1000);

    /// Arm an HDMA channel during V-Blank (as a ROM does), then cross into
    /// the next frame so the channel gets its once-per-frame init.
    fn arm_hdma(rsnes: &mut RSnesCore, dmap: u8, bbad: u8, table: &[u8]) {
        advance_core_to_scanline(rsnes, VBLANK_START_LINE);

        fill_wram(rsnes, HDMA_TABLE, table);
        configure_channel(rsnes, 0, dmap, bbad, HDMA_TABLE, 0);
        write_reg(rsnes, 0x420C, 0b0000_0001);

        advance_core_to_scanline(rsnes, 0);
    }

    /// Advance to `line` and past dot 278, so that line's HDMA pass has run.
    fn settle_line(rsnes: &mut RSnesCore, line: u16) {
        if !(rsnes.ppu.scanline == line && rsnes.ppu.h_cycles == 0) {
            advance_core_to_scanline(rsnes, line);
        }
        tick_core(rsnes, HDMA_START_DOT as u64 * 4 + 128);
    }

    /// A transfer is requested in the H-Blank of every visible scanline
    /// while HDMAEN is non-zero.
    #[test]
    fn test_hdma_transfer_requested_during_visible_lines() {
        let mut rsnes = TestRsnesCore::new();
        rsnes.bus.io.hdmaen = 0b0000_0001;

        tick_core(&mut rsnes, HDMA_START_DOT as u64 * 4);
        assert!(rsnes.dma.hdma_pending || rsnes.dma.state == DmaState::Hdma);
    }

    /// HDMA never runs during V-Blank — that window belongs to the ROM.
    /// Reaching the end without panicking is the assertion.
    #[test]
    fn test_hdma_not_requested_during_vblank() {
        let mut rsnes = TestRsnesCore::new();
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        rsnes.bus.io.hdmaen = 0b0000_0001;

        tick_core(&mut rsnes, MASTER_CYCLES_PER_SCANLINE as u64);
    }

    /// Channels are re-initialised at the top of each frame.
    #[test]
    fn test_hdma_init_requested_at_frame_start() {
        let mut rsnes = TestRsnesCore::new();
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);
        rsnes.bus.io.hdmaen = 0b0000_0001;

        advance_core_to_scanline(&mut rsnes, 0);
        assert!(rsnes.dma.hdma_init || rsnes.dma.state == DmaState::Hdma);
    }

    /// Nothing is requested when HDMAEN is clear.
    #[test]
    fn test_no_hdma_when_disabled() {
        let mut rsnes = TestRsnesCore::new();
        advance_core_to_scanline(&mut rsnes, 10);
    }

    /// Transfer mode 1 writes alternating bytes to $2118/$2119, so four
    /// source bytes become two VRAM words in source order.
    #[test]
    fn test_dma_mode1_builds_vram_words_in_order() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0x11, 0x22, 0x33, 0x44]);
        vram_word_mode(&mut rsnes);

        configure_channel(&mut rsnes, 0, 0x01, 0x18, src, 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.vram.memory[0], 0x2211);
        assert_eq!(rsnes.ppu.vram.memory[1], 0x4433);
    }

    /// DMAP bit 4 walks the A-bus backwards, so the same four bytes arrive
    /// reversed.
    #[test]
    fn test_dma_decrement_reverses_source_order() {
        let mut rsnes = TestRsnesCore::new();
        let base = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, base, &[0x11, 0x22, 0x33, 0x44]);
        vram_word_mode(&mut rsnes);

        // Start at the last byte and walk down.
        configure_channel(&mut rsnes, 0, 0x11, 0x18, snes_addr!(0x7E:0x1003), 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.vram.memory[0], 0x3344);
        assert_eq!(rsnes.ppu.vram.memory[1], 0x1122);
    }

    /// DMAP bit 3 pins the A-bus, so the same byte is read every time.
    /// Bit 3 wins even when bit 4 also asks for a decrement.
    #[test]
    fn test_dma_fixed_source_repeats_one_byte() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0xAB, 0xCD, 0xEF, 0x01]);
        vram_word_mode(&mut rsnes);

        // bits 4 and 3 both set: fixed takes priority.
        configure_channel(&mut rsnes, 0, 0x19, 0x18, src, 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.vram.memory[0], 0xABAB);
        assert_eq!(rsnes.ppu.vram.memory[1], 0xABAB);
        assert_eq!(channel_a1t(&mut rsnes, 0).addr, 0x1000);
    }

    /// DMAP bit 7 reverses the direction: the B-bus is read and the A-bus
    /// written. Reading $213B walks CGRAM, so four reads land four bytes.
    #[test]
    fn test_dma_b_to_a_writes_into_wram() {
        let mut rsnes = TestRsnesCore::new();

        // Seed two CGRAM colours, then rewind the CGRAM address.
        write_reg(&mut rsnes, 0x2121, 0x00);
        for b in [0xEF, 0x3A, 0xCD, 0x12] {
            write_reg(&mut rsnes, 0x2122, b);
        }
        write_reg(&mut rsnes, 0x2121, 0x00);

        let dst = snes_addr!(0x7E:0x1000);
        configure_channel(&mut rsnes, 0, 0x80, 0x3B, dst, 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.bus.wram.read(snes_addr!(0x7E:0x1000)), 0xEF);
        assert_eq!(rsnes.bus.wram.read(snes_addr!(0x7E:0x1001)) & 0x7F, 0x3A);
        assert_eq!(rsnes.bus.wram.read(snes_addr!(0x7E:0x1002)), 0xCD);
        assert_eq!(rsnes.bus.wram.read(snes_addr!(0x7E:0x1003)) & 0x7F, 0x12);
    }

    /// A ROM reading $43n2/$43n5 afterwards sees the source advanced by the
    /// byte count and the counter drained to zero.
    #[test]
    fn test_dma_leaves_source_advanced_and_count_zero() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0; 8]);

        configure_channel(&mut rsnes, 0, 0x00, 0x26, src, 8);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(channel_a1t(&mut rsnes, 0).addr, 0x1008);
        assert_eq!(channel_das(&mut rsnes, 0), 0);
    }

    /// The bank byte is never carried into: the A-bus wraps inside its bank.
    #[test]
    fn test_dma_source_wraps_inside_its_bank() {
        let mut rsnes = TestRsnesCore::new();
        rsnes.bus.wram.write(snes_addr!(0x7E:0xFFFF), 0xAA);
        rsnes.bus.wram.write(snes_addr!(0x7E:0x0000), 0xBB);
        vram_word_mode(&mut rsnes);

        configure_channel(&mut rsnes, 0, 0x01, 0x18, snes_addr!(0x7E:0xFFFF), 2);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.vram.memory[0], 0xBBAA);
        assert_eq!(channel_a1t(&mut rsnes, 0).bank, 0x7E);
    }

    /// A count of zero means the full 64 KiB, not nothing. The source walks
    /// exactly one lap of its bank.
    #[test]
    fn test_dma_count_zero_transfers_64k() {
        let mut rsnes = TestRsnesCore::new();

        configure_channel(&mut rsnes, 0, 0x08, 0x26, snes_addr!(0x7E:0x0000), 0);
        let cycles = run_dma(&mut rsnes, 0b0000_0001, 1_000_000);

        assert_eq!(channel_a1t(&mut rsnes, 0).addr, 0x0000);
        assert!(cycles >= 0x10000 * BYTE_COST as u64);
    }

    /// Channels run lowest-numbered first, one fully at a time, so the
    /// higher channel's byte is the one left in the destination.
    #[test]
    fn test_dma_channels_run_lowest_first() {
        let mut rsnes = TestRsnesCore::new();
        fill_wram(&mut rsnes, snes_addr!(0x7E:0x1000), &[0xAA]);
        fill_wram(&mut rsnes, snes_addr!(0x7E:0x2000), &[0xBB]);

        configure_channel(&mut rsnes, 0, 0x00, 0x26, snes_addr!(0x7E:0x1000), 1);
        configure_channel(&mut rsnes, 1, 0x00, 0x26, snes_addr!(0x7E:0x2000), 1);
        run_dma(&mut rsnes, 0b0000_0011, 10_000);

        assert_eq!(rsnes.ppu.regs.wh0, 0xBB);
        assert_eq!(channel_a1t(&mut rsnes, 0).addr, 0x1001);
        assert_eq!(channel_a1t(&mut rsnes, 1).addr, 0x2001);
    }

    /// Channels not named in $420B are untouched.
    #[test]
    fn test_dma_ignores_disabled_channels() {
        let mut rsnes = TestRsnesCore::new();

        configure_channel(&mut rsnes, 0, 0x00, 0x26, snes_addr!(0x7E:0x1000), 4);
        configure_channel(&mut rsnes, 1, 0x00, 0x26, snes_addr!(0x7E:0x2000), 4);
        run_dma(&mut rsnes, 0b0000_0010, 10_000);

        assert_eq!(channel_a1t(&mut rsnes, 0).addr, 0x1000);
        assert_eq!(channel_das(&mut rsnes, 0), 4);
        assert_eq!(channel_a1t(&mut rsnes, 1).addr, 0x2004);
    }

    /// Every byte costs 8 master cycles. Differencing two transfer sizes
    /// isolates the rate from the fixed start and per-channel overheads.
    #[test]
    fn test_dma_costs_eight_master_cycles_per_byte() {
        let mut short = TestRsnesCore::new();
        configure_channel(&mut short, 0, 0x08, 0x26, snes_addr!(0x7E:0x1000), 4);
        let short_cycles = run_dma(&mut short, 0b0000_0001, 10_000);

        let mut long = TestRsnesCore::new();
        configure_channel(&mut long, 0, 0x08, 0x26, snes_addr!(0x7E:0x1000), 20);
        let long_cycles = run_dma(&mut long, 0b0000_0001, 10_000);

        assert_eq!(long_cycles - short_cycles, 16 * BYTE_COST as u64);
    }

    /// A non-repeat entry writes once and holds the value for its whole
    /// line count; the next entry takes over when the count runs out.
    #[test]
    fn test_hdma_non_repeat_holds_value_across_lines() {
        let mut rsnes = TestRsnesCore::new();
        // 2 lines of 0x10, 2 lines of 0x20, terminate.
        arm_hdma(&mut rsnes, 0x00, 0x26, &[0x02, 0x10, 0x02, 0x20, 0x00]);

        settle_line(&mut rsnes, 0);
        assert_eq!(rsnes.ppu.regs.wh0, 0x10);

        settle_line(&mut rsnes, 1);
        assert_eq!(rsnes.ppu.regs.wh0, 0x10);

        settle_line(&mut rsnes, 2);
        assert_eq!(rsnes.ppu.regs.wh0, 0x20);

        settle_line(&mut rsnes, 3);
        assert_eq!(rsnes.ppu.regs.wh0, 0x20);
    }

    /// Bit 7 of the line count means "a fresh byte every line", which is
    /// how a gradient is drawn.
    #[test]
    fn test_hdma_repeat_writes_a_new_byte_per_line() {
        let mut rsnes = TestRsnesCore::new();
        // repeat, 3 lines, three data bytes, terminate.
        arm_hdma(&mut rsnes, 0x00, 0x26, &[0x83, 0x11, 0x22, 0x33, 0x00]);

        for (line, expected) in [(0, 0x11), (1, 0x22), (2, 0x33)] {
            settle_line(&mut rsnes, line);
            assert_eq!(rsnes.ppu.regs.wh0, expected);
        }
    }

    /// A line count of $00 ends the channel for the rest of the frame; the
    /// destination keeps whatever was last written.
    #[test]
    fn test_hdma_zero_count_terminates_for_the_frame() {
        let mut rsnes = TestRsnesCore::new();
        arm_hdma(&mut rsnes, 0x00, 0x26, &[0x01, 0x77, 0x00, 0x99, 0x99]);

        settle_line(&mut rsnes, 0);
        assert_eq!(rsnes.ppu.regs.wh0, 0x77);

        // Line 1 hits the terminator. Nothing past it is ever read.
        for line in 1..VBLANK_START_LINE {
            settle_line(&mut rsnes, line);
            assert_eq!(rsnes.ppu.regs.wh0, 0x77);
        }
    }

    /// The table pointer rewinds at the top of each frame, so the same
    /// sequence plays again.
    #[test]
    fn test_hdma_table_restarts_every_frame() {
        let mut rsnes = TestRsnesCore::new();
        arm_hdma(&mut rsnes, 0x00, 0x26, &[0x83, 0x11, 0x22, 0x33, 0x00]);

        settle_line(&mut rsnes, 0);
        assert_eq!(rsnes.ppu.regs.wh0, 0x11);
        settle_line(&mut rsnes, 1);
        assert_eq!(rsnes.ppu.regs.wh0, 0x22);
        settle_line(&mut rsnes, 2);
        assert_eq!(rsnes.ppu.regs.wh0, 0x33);
        settle_line(&mut rsnes, 3);
        assert_eq!(rsnes.ppu.regs.wh0, 0x33);

        // Cross into the next frame.
        advance_core_to_scanline(&mut rsnes, 0);
        settle_line(&mut rsnes, 0);
        assert_eq!(rsnes.ppu.regs.wh0, 0x11);
    }

    /// DMAP bit 6 makes the entry carry a pointer instead of data; the
    /// bytes come from DASB:DAS and that pointer advances as they are used.
    #[test]
    fn test_hdma_indirect_streams_from_a_separate_pointer() {
        let mut rsnes = TestRsnesCore::new();
        // repeat, 2 lines, pointer to $7E:2000, terminate.
        arm_hdma(&mut rsnes, 0x40, 0x26, &[0x82, 0x00, 0x20, 0x00]);
        fill_wram(&mut rsnes, snes_addr!(0x7E:0x2000), &[0x5A, 0xA5]);
        // DASB selects the bank the indirect data lives in.
        write_reg(&mut rsnes, 0x4307, 0x7E);

        settle_line(&mut rsnes, 0);
        assert_eq!(rsnes.ppu.regs.wh0, 0x5A);

        settle_line(&mut rsnes, 1);
        assert_eq!(rsnes.ppu.regs.wh0, 0xA5);
    }

    /// HDMA runs on visible lines only. The destination stops changing
    /// while in V-BLANK even with entries left in the table.
    #[test]
    fn test_hdma_stops_at_vblank() {
        let mut rsnes = TestRsnesCore::new();

        // Two repeat entries of 127 lines each, data counting upward, so the
        // destination records how many transfers have happened.
        let mut table = vec![0xFF];
        table.extend(1..=127);
        table.push(0xFF);
        table.extend(128..=254);
        table.push(0x00);
        arm_hdma(&mut rsnes, 0x00, 0x26, &table);

        settle_line(&mut rsnes, VBLANK_START_LINE - 1);
        let last_visible = rsnes.ppu.regs.wh0;
        assert_eq!(last_visible, 225);

        settle_line(&mut rsnes, VBLANK_START_LINE + 3);
        assert_eq!(rsnes.ppu.regs.wh0, last_visible);
    }

    /// Nothing happens while HDMAEN is clear, however the channel is set up.
    #[test]
    fn test_no_hdma_when_hdmaen_is_clear() {
        let mut rsnes = TestRsnesCore::new();
        advance_core_to_scanline(&mut rsnes, VBLANK_START_LINE);

        fill_wram(&mut rsnes, HDMA_TABLE, &[0x83, 0x11, 0x22, 0x00]);
        configure_channel(&mut rsnes, 0, 0x00, 0x26, HDMA_TABLE, 0);
        // HDMAEN deliberately left at 0.

        advance_core_to_scanline(&mut rsnes, 0);
        settle_line(&mut rsnes, 5);

        assert_eq!(rsnes.ppu.regs.wh0, 0);
    }

    /// A channel enabled in both $420B and $420C loses its general-purpose
    /// transfer outright: HDMA wins and the transfer is aborted, not queued.
    #[test]
    fn test_hdma_channel_cancels_its_general_purpose_transfer() {
        let mut rsnes = TestRsnesCore::new();
        arm_hdma(&mut rsnes, 0x00, 0x26, &[0x7F, 0x55, 0x00]);

        // Repoint the same channel at a bulk transfer and request it.
        fill_wram(&mut rsnes, snes_addr!(0x7E:0x2000), &[0xAA; 8]);
        configure_channel(&mut rsnes, 0, 0x00, 0x26, snes_addr!(0x7E:0x2000), 8);
        write_reg(&mut rsnes, 0x420B, 0b0000_0001);

        tick_core(&mut rsnes, 2_000);

        assert_eq!(rsnes.bus.io.mdmaen, 0);
        assert_eq!(channel_a1t(&mut rsnes, 0).addr, 0x2000);
    }

    /// Mode 0 writes every byte to the same B-bus register.
    #[test]
    fn test_dma_pattern_mode0_single_register() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0x11, 0x22, 0x33, 0x44]);

        // Increment after $2118 (VMAIN bit 7 clear).
        write_reg(&mut rsnes, 0x2115, 0x00);
        write_reg(&mut rsnes, 0x2116, 0x00);
        write_reg(&mut rsnes, 0x2117, 0x00);

        configure_channel(&mut rsnes, 0, 0x00, 0x18, src, 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.vram.memory[0], 0x0011);
        assert_eq!(rsnes.ppu.vram.memory[1], 0x0022);
        assert_eq!(rsnes.ppu.vram.memory[2], 0x0033);
        assert_eq!(rsnes.ppu.vram.memory[3], 0x0044);
    }

    /// Mode 2 writes pairs to the same register, so the second byte of
    /// each pair overwrites the first.
    #[test]
    fn test_dma_pattern_mode2_pairs_to_one_register() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0x11, 0x22, 0x33, 0x44]);

        configure_channel(&mut rsnes, 0, 0x02, 0x26, src, 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.regs.wh0, 0x44);
    }

    /// Mode 3 writes two bytes to the base register, then two to base+1.
    #[test]
    fn test_dma_pattern_mode3_two_then_two() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0x11, 0x22, 0x33, 0x44]);

        configure_channel(&mut rsnes, 0, 0x03, 0x26, src, 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.regs.wh0, 0x22);
        assert_eq!(rsnes.ppu.regs.wh1, 0x44);
    }

    /// Mode 4 walks four consecutive registers, one byte each.
    #[test]
    fn test_dma_pattern_mode4_four_consecutive_registers() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0x11, 0x22, 0x33, 0x44]);

        configure_channel(&mut rsnes, 0, 0x04, 0x26, src, 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.regs.wh0, 0x11);
        assert_eq!(rsnes.ppu.regs.wh1, 0x22);
        assert_eq!(rsnes.ppu.regs.wh2, 0x33);
        assert_eq!(rsnes.ppu.regs.wh3, 0x44);
    }

    /// Mode 5 alternates base / base+1 in pairs.
    #[test]
    fn test_dma_pattern_mode5_alternating_pairs() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0x11, 0x22, 0x33, 0x44]);

        configure_channel(&mut rsnes, 0, 0x05, 0x26, src, 4);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.regs.wh0, 0x33);
        assert_eq!(rsnes.ppu.regs.wh1, 0x44);
    }

    /// Modes 6 and 7 are documented aliases of 2 and 3.
    #[test]
    fn test_dma_pattern_modes_6_and_7_alias_2_and_3() {
        let src = snes_addr!(0x7E:0x1000);

        let mut six = TestRsnesCore::new();
        fill_wram(&mut six, src, &[0x11, 0x22, 0x33, 0x44]);
        configure_channel(&mut six, 0, 0x06, 0x26, src, 4);
        run_dma(&mut six, 0b0000_0001, 10_000);
        assert_eq!(six.ppu.regs.wh0, 0x44, "mode 6 behaves as mode 2");

        let mut seven = TestRsnesCore::new();
        fill_wram(&mut seven, src, &[0x11, 0x22, 0x33, 0x44]);
        configure_channel(&mut seven, 0, 0x07, 0x26, src, 4);
        run_dma(&mut seven, 0b0000_0001, 10_000);
        assert_eq!(seven.ppu.regs.wh0, 0x22, "mode 7 behaves as mode 3: $2126");
        assert_eq!(seven.ppu.regs.wh1, 0x44, "mode 7 behaves as mode 3: $2127");
    }

    /// The pattern cycles rather than truncating: eight bytes in mode 1 fill
    /// four VRAM words, not two.
    #[test]
    fn test_dma_pattern_repeats_beyond_its_length() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(
            &mut rsnes,
            src,
            &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
        );
        vram_word_mode(&mut rsnes);

        configure_channel(&mut rsnes, 0, 0x01, 0x18, src, 8);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.vram.memory[0], 0x2211);
        assert_eq!(rsnes.ppu.vram.memory[1], 0x4433);
        assert_eq!(rsnes.ppu.vram.memory[2], 0x6655);
        assert_eq!(rsnes.ppu.vram.memory[3], 0x8877);
    }

    /// A transfer that stops mid-unit still moves exactly DAS bytes.
    #[test]
    fn test_dma_stops_mid_pattern_on_odd_count() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0x11, 0x22, 0x33]);

        configure_channel(&mut rsnes, 0, 0x04, 0x26, src, 3);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.regs.wh0, 0x11);
        assert_eq!(rsnes.ppu.regs.wh1, 0x22);
        assert_eq!(rsnes.ppu.regs.wh2, 0x33);
        assert_eq!(rsnes.ppu.regs.wh3, 0);
        assert_eq!(channel_das(&mut rsnes, 0), 0);
    }

    /// BBAD wraps inside the $21xx page rather than spilling into $22xx.
    #[test]
    fn test_dma_b_address_wraps_within_the_page() {
        let mut rsnes = TestRsnesCore::new();
        let src = snes_addr!(0x7E:0x1000);
        fill_wram(&mut rsnes, src, &[0x11, 0x22]);

        configure_channel(&mut rsnes, 0, 0x01, 0xFF, src, 2);
        run_dma(&mut rsnes, 0b0000_0001, 10_000);

        assert_eq!(rsnes.ppu.regs.inidisp, 0x22);
    }
}

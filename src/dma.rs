use crate::rsnes::RSnesCore;
use common::snes_address::SnesAddress;

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
        SnesAddress {
            bank: 0x00,
            addr: 0x2100 + bbad.wrapping_add(offset) as u16,
        }
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

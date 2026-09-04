use common::snes_address::SnesAddress;

/// Master cycles per byte moved. Constant regardless of MEMSEL —
/// FastROM does not speed up DMA.
pub const BYTE_COST: u32 = 8;
/// One-off cost when a DMA transfer begins.
pub const DMA_START_COST: u32 = 8;
/// Cost when a DMA channel begins, paid once per channel.
pub const DMA_CHANNEL_COST: u32 = 8;
/// Cost of resuming a DMA that HDMA preempted.
pub const DMA_RESUME_COST: u32 = 8;
/// Per-scanline cost when any HDMA channel is enabled.
pub const HDMA_LINE_COST: u32 = 18;
/// Per-channel cost when a channel does anything on a line.
pub const HDMA_CHANNEL_COST: u32 = 8;
/// Extra cost when a channel loads a new table entry.
pub const HDMA_RELOAD_COST: u32 = 16;
/// As above, for indirect channels (two more table bytes).
pub const HDMA_RELOAD_INDIRECT_COST: u32 = 24;
pub const HDMA_INDIRECT_LOAD_COST: u32 = 16;
pub const HDMA_INIT_DIRECT_COST: u32 = 8;
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

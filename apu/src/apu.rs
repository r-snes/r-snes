use crate::{cpu::Spc700, memory::Memory, timers::Timers};

// The SPC700 CPU runs at 1.024 MHz.
// The DSP produces one output sample every 32 CPU cycles (32 kHz).
// We count CPU cycles and only tick the DSP when this threshold is reached.
const DSP_CYCLES_PER_SAMPLE: u32 = 32;

/// High-level emulation of the SPC700 IPL boot ROM.
///
/// The real hardware boots the SPC700 into a 64-byte mask ROM at
/// $FFC0-$FFFF which announces itself on the communication ports and
/// implements the standard upload protocol every game uses to load its
/// sound driver. We don't ship that ROM; instead this state machine
/// reproduces its externally visible behaviour:
///
/// 1. Announce: port0 = $AA, port1 = $BB. Main CPU polls $2140/$2141.
/// 2. Main CPU writes target addr to $2142/$2143, non-zero to $2141
///    (transfer) and $CC to $2140. We ack by echoing $CC on port0.
/// 3. Per byte: main CPU writes data to $2141, then the byte index to
///    $2140, and waits for the index echoed back on $2140. We store the
///    byte at addr++ and echo.
/// 4. New command: main CPU writes a value >= index+2 to $2140 with a
///    new addr in $2142/$2143. Non-zero $2141 = next transfer block
///    (byte index restarts at 0); zero = execute: we point the SPC700's
///    PC at the address and hand control to the real CPU core.
///
/// While the IPL is active, `Apu::step` runs this state machine instead
/// of `Spc700::step` — just like the real chip, which is busy executing
/// IPL code during this phase. Uploaded code can jump back to $FFC0 to
/// re-run the boot ROM for another upload; `Apu::step` detects this and
/// re-arms the state machine (see `reenter_ipl`).
#[derive(Debug, Clone, Copy, PartialEq)]
enum IplHle {
    /// Announcing $AA/$BB, waiting for the $CC start command.
    AwaitStart,
    /// Receiving data bytes for the block being uploaded.
    Transfer { addr: u16, index: u8 },
    /// Upload finished; the SPC700 core is executing uploaded code.
    Done,
}

pub struct Apu {
    pub cpu:    Spc700,
    pub memory: Memory,   // Memory::dsp is the *only* Dsp — there is no separate field
    pub timers: Timers,

    /// Total CPU cycles elapsed since APU creation.
    pub cycles: u64,

    /// Counts CPU cycles since the last DSP tick.
    /// Resets to 0 every DSP_CYCLES_PER_SAMPLE cycles.
    dsp_cycles: u32,

    /// HLE IPL boot state (see IplHle docs).
    ipl: IplHle,
}

impl Apu {
    /// The SPC700 runs at a fixed 1.024 MHz, derived from its own
    /// independent 24.576 MHz crystal -- it is NOT phase-locked to the
    /// SNES master clock. Real hardware has two unsynchronized oscillators;
    /// RSnes approximates the average ratio with an integer cycle-debt
    /// accumulator (see RSnes::update_apu_cycles).
    pub const CLOCK_HZ: u64 = 1_024_000;

    pub fn new() -> Self {
        let mut apu = Self {
            cpu:        Spc700::new(),
            memory:     Memory::new(),
            timers:     Timers::new(),
            cycles:     0,
            dsp_cycles: 0,
            ipl:        IplHle::AwaitStart,
        };

        // Load the reset vector and initialise SP so the CPU starts correctly.
        apu.cpu.reset(&mut apu.memory);

        // Run the (HLE) IPL boot sequence: post-boot register state and
        // the $AA/$BB announce on the ports.
        apu.ipl_boot();

        apu
    }

    /// Reproduce the externally visible side effects of the IPL boot ROM
    /// starting to run, and arm the HLE state machine. Called at power-on
    /// and again whenever uploaded code jumps back to $FFC0 to request
    /// another upload (how multi-chunk transfers chain on real hardware).
    fn ipl_boot(&mut self) {
        // The real IPL's first acts are `mov x,#$EF / mov sp,x` and a loop
        // clearing zero page $00-$EF. Uploaded code (and the spc test
        // suite) assumes this post-boot state: test #0081 places its ADDW
        // operand at $01FF, relying on the stack starting at $01EF and
        // growing downward, never reaching $01FF.
        self.cpu.regs.sp = 0xEF;
        self.memory.ram[0x00..=0xEF].fill(0);

        // Announce: the main CPU spins on $2140/$2141 until it sees
        // $AA/$BB — this is the handshake every upload starts with.
        self.memory.port_out[0] = 0xAA;
        self.memory.port_out[1] = 0xBB;

        self.ipl = IplHle::AwaitStart;
    }

    /// True while the HLE IPL owns the SPC700 (before the execute command).
    pub fn ipl_active(&self) -> bool {
        self.ipl != IplHle::Done
    }

    /// Re-arm the HLE IPL after uploaded code jumps back into the boot
    /// ROM region. Replicates the real IPL's startup side effects, which
    /// run unconditionally from the top on every entry:
    ///   - SP reset to $EF
    ///   - zero page $01-$EF cleared (the real clear loop stops before
    ///     $00; note it targets page 1 instead if the P flag is set — a
    ///     hardware quirk we deliberately don't model, since well-behaved
    ///     drivers clear P before jumping to $FFC0)
    ///   - $AA/$BB announced on ports 0/1
    fn reenter_ipl(&mut self) {
        eprintln!(
            "[apu ipl] re-entered at pc={:#06x} — announcing, awaiting next upload",
            self.cpu.regs.pc
        );
        self.cpu.regs.sp = 0xEF;
        self.memory.ram[0x01..=0xEF].fill(0);
        self.memory.port_out[0] = 0xAA;
        self.memory.port_out[1] = 0xBB;
        self.ipl = IplHle::AwaitStart;
    }

    /// One tick of the HLE IPL state machine. Called from `step` in place
    /// of `Spc700::step` while the boot upload is in progress. Polls the
    /// input ports exactly like the real IPL's wait loops do.
    fn ipl_step(&mut self) {
        match self.ipl {
            IplHle::AwaitStart => {
                if self.memory.port_in[0] == 0xCC {
                    let addr = u16::from_le_bytes([
                        self.memory.port_in[2],
                        self.memory.port_in[3],
                    ]);
                    // Ack the start command by echoing $CC.
                    self.memory.port_out[0] = 0xCC;

                    if self.memory.port_in[1] != 0 {
                        eprintln!("[apu ipl] start command: uploading block to {addr:#06x}");
                        self.ipl = IplHle::Transfer { addr, index: 0 };
                    } else {
                        eprintln!("[apu ipl] start command: direct execute at {addr:#06x}");
                        self.cpu.regs.pc = addr;
                        self.ipl = IplHle::Done;
                    }
                }
            }

            IplHle::Transfer { addr, index } => {
                let f4 = self.memory.port_in[0];
                // Same comparison the real IPL performs: negative delta =
                // stale value from the previous byte (keep waiting),
                // zero = next data byte, positive = new command.
                let delta = f4.wrapping_sub(index) as i8;

                if delta == 0 {
                    // Data byte: main CPU wrote data to port1 *before*
                    // bumping the index on port0, so port1 is valid now.
                    let data = self.memory.port_in[1];
                    self.memory.ram[addr as usize] = data;
                    self.memory.port_out[0] = index; // ack by echoing index
                    self.ipl = IplHle::Transfer {
                        addr:  addr.wrapping_add(1),
                        index: index.wrapping_add(1),
                    };
                } else if delta > 0 {
                    // New command: next block, or execute.
                    let new_addr = u16::from_le_bytes([
                        self.memory.port_in[2],
                        self.memory.port_in[3],
                    ]);
                    self.memory.port_out[0] = f4; // ack the command byte

                    if self.memory.port_in[1] != 0 {
                        eprintln!("[apu ipl] next block at {new_addr:#06x}");
                        self.ipl = IplHle::Transfer { addr: new_addr, index: 0 };
                    } else {
                        eprintln!("[apu ipl] execute at {new_addr:#06x} — handing off to SPC700");
                        self.cpu.regs.pc = new_addr;
                        self.ipl = IplHle::Done;
                    }
                }
                // delta < 0: main CPU hasn't advanced yet; keep waiting.
            }

            IplHle::Done => unreachable!("guarded by caller"),
        }
    }

    /// Step the APU forward by `cycles` CPU cycles.
    ///
    /// Each call ticks:
    ///   - The SPC700 CPU  (every cycle; the HLE IPL while boot upload runs)
    ///   - The timers      (every cycle)
    ///   - The DSP         (once every 32 cycles → 32 kHz)
    ///
    /// All DSP access goes through `self.memory.dsp`; there is no
    /// separate Dsp field on Apu.
    pub fn step(&mut self, cycles: u32) {
        for _ in 0..cycles {
            if self.ipl_active() {
                // The real chip spends this time executing IPL code from
                // the boot ROM; we run the HLE state machine instead.
                self.ipl_step();
            } else if self.cpu.regs.pc >= 0xFFC0 {
                // Jumping into $FFC0-$FFFF re-runs the boot ROM: drivers
                // do this to request another upload (the spc test suite
                // does it between chunks; games do it between tracks).
                self.reenter_ipl();
            } else {
                self.cpu.step(&mut self.memory);
            }

            self.timers.step(&mut self.memory);

            self.dsp_cycles += 1;
            if self.dsp_cycles >= DSP_CYCLES_PER_SAMPLE {
                self.dsp_cycles = 0;
                self.memory.dsp.step(&self.memory.ram);
            }

            self.cycles += 1;
        }
    }

    /// Generate `num_samples` stereo output samples.
    ///
    /// Steps the APU internally for each sample so that CPU, timers, and DSP
    /// all advance in lock-step.  Returns a `Vec` of `(left, right)` pairs.
    pub fn render_audio(&mut self, num_samples: usize) -> Vec<(i16, i16)> {
        let mut buf = Vec::with_capacity(num_samples);

        for _ in 0..num_samples {
            // Advance the full APU by one DSP period (32 CPU cycles = 1 sample).
            self.step(DSP_CYCLES_PER_SAMPLE);

            // Collect the stereo output from the DSP as an explicit (L, R) pair.
            buf.push(self.memory.dsp.render_audio_single());
        }

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the IPL protocol from the "main CPU" side, the way a game's
    /// boot code would through $2140-$2143, and verify the upload lands
    /// in ARAM and execution starts at the requested entry point.
    #[test]
    fn test_ipl_hle_upload_and_execute() {
        let mut apu = Apu::new();

        // 1. Announce
        assert_eq!(apu.memory.cpu_port_read(0), 0xAA);
        assert_eq!(apu.memory.cpu_port_read(1), 0xBB);

        // 2. Start command: upload to $0200
        apu.memory.cpu_port_write(2, 0x00);
        apu.memory.cpu_port_write(3, 0x02);
        apu.memory.cpu_port_write(1, 0x01); // non-zero = transfer
        apu.memory.cpu_port_write(0, 0xCC);
        apu.step(2);
        assert_eq!(apu.memory.cpu_port_read(0), 0xCC, "IPL must ack $CC");

        // 3. Upload three bytes
        for (i, byte) in [0xDE_u8, 0xAD, 0xBE].iter().enumerate() {
            apu.memory.cpu_port_write(1, *byte);
            apu.memory.cpu_port_write(0, i as u8);
            apu.step(2);
            assert_eq!(apu.memory.cpu_port_read(0), i as u8, "IPL must echo index");
        }
        assert_eq!(&apu.memory.ram[0x0200..0x0203], &[0xDE, 0xAD, 0xBE]);

        // 4. Execute command: index jumped by >= 2, port1 = 0, addr = $0200
        apu.memory.cpu_port_write(2, 0x00);
        apu.memory.cpu_port_write(3, 0x02);
        apu.memory.cpu_port_write(1, 0x00); // zero = execute
        apu.memory.cpu_port_write(0, 0x05); // last index was 2; 2 + >=2
        apu.step(2);

        assert!(!apu.ipl_active(), "IPL should have handed off");
        assert_eq!(apu.cpu.regs.pc, 0x0200, "SPC700 must start at the entry point");
    }
}

use crate::{cpu::Spc700, memory::Memory, timers::Timers};

// The SPC700 CPU runs at 1.024 MHz.
// The DSP produces one output sample every 32 CPU cycles (32 kHz).
// We count CPU cycles and only tick the DSP when this threshold is reached.
const DSP_CYCLES_PER_SAMPLE: u32 = 32;

/// IPL boot-protocol trace, printed only in debug builds. Reads as a
/// boot log (announce, upload blocks, execute) and is the first thing to
/// check when a ROM hangs during the CPU<->APU handshake.
macro_rules! ipl_trace {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!($($arg)*);
    };
}

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
    /// Boot-time before the announce, modeling the real IPL's SP init and
    /// zero-page clear loop (~1000 cycles). This delay is load-bearing:
    /// it keeps whatever the previous code left on port_out — e.g. the
    /// spc test suite's "chunk complete" signal — visible to the main CPU
    /// long enough to be sampled, instead of stomping it with $AA on the
    /// very next APU cycle after a jump to $FFC0.
    BootDelay { cycles_left: u16 },
    /// Announcing $AA/$BB, waiting for the $CC start command.
    AwaitStart,
    /// Receiving data bytes for the block being uploaded.
    Transfer { addr: u16, index: u8 },
    /// Execute command received and acked; holding the ack stable on
    /// port 0 for a grace period before the uploaded code starts. This
    /// is the only unsynchronized handoff in the protocol: once the
    /// uploaded program runs it may write $F4 immediately, and if that
    /// happens before the main CPU samples the execute ack, the main
    /// CPU waits forever for a value that's already gone. The real IPL
    /// provides a small window here via its jump-sequence overhead; we
    /// provide a more generous one.
    ExecDelay { cycles_left: u16, entry: u16 },
}

/// SPC700 cycles the HLE IPL spends "booting" before announcing $AA/$BB,
/// approximating the real boot ROM's SP init + zero-page clear (~1 ms).
const IPL_BOOT_CYCLES: u16 = 1024;

/// SPC700 cycles the execute ack stays stable on port 0 before the
/// uploaded code starts running (see IplHle::ExecDelay).
const IPL_EXEC_DELAY_CYCLES: u16 = 256;

pub struct Apu {
    pub cpu: Spc700,
    pub memory: Memory,
    pub timers: Timers,

    /// Total CPU cycles elapsed since APU creation.
    pub cycles: u64,

    /// Counts CPU cycles since the last DSP tick.
    /// Resets to 0 every DSP_CYCLES_PER_SAMPLE cycles.
    dsp_cycles: u32,

    /// HLE IPL boot state. `None` once the upload has
    /// finished and the SPC700 core is executing uploaded code.
    ipl: Option<IplHle>,
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
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
            cpu: Spc700::new(),
            memory: Memory::new(),
            timers: Timers::new(),
            cycles: 0,
            dsp_cycles: 0,
            ipl: Some(IplHle::BootDelay {
                cycles_left: IPL_BOOT_CYCLES,
            }),
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

        // Do NOT announce $AA/$BB yet: the real boot ROM spends ~1000
        // cycles on the init above before touching the ports, and that
        // delay is protocol-critical — it leaves the previous code's
        // port_out values (e.g. a "chunk complete" signal the main CPU
        // is polling for) visible long enough to be sampled. Announcing
        // on the very next cycle creates a race where the main CPU can
        // miss that signal and deadlock. The announce happens when
        // BootDelay elapses, in ipl_step.
        self.ipl = Some(IplHle::BootDelay {
            cycles_left: IPL_BOOT_CYCLES,
        });
    }

    /// True while the HLE IPL owns the SPC700 (before the execute command).
    pub fn ipl_active(&self) -> bool {
        self.ipl.is_some()
    }

    /// Skip the HLE IPL boot entirely, handing the core to the SPC700
    /// immediately. Intended for tests and tools that construct an `Apu`
    /// and execute code directly, without performing the main-CPU upload
    /// protocol. The announce values are still placed on the ports so
    /// anything checking for the $AA/$BB handshake keeps working, and SP
    /// keeps its post-IPL value of $EF from `new`.
    pub fn skip_ipl_boot(&mut self) {
        self.memory.port_out[0] = 0xAA;
        self.memory.port_out[1] = 0xBB;
        self.ipl = None;
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
        ipl_trace!(
            "[apu ipl] re-entered at pc={:#06x} — booting, will announce shortly",
            self.cpu.regs.pc
        );
        self.ipl_boot();
    }

    /// One tick of the HLE IPL state machine. Called from `step` in place
    /// of `Spc700::step` while the boot upload is in progress. Polls the
    /// input ports exactly like the real IPL's wait loops do.
    ///
    /// Takes the current (necessarily active) state and returns the next
    /// one; `None` means the IPL has handed control to the SPC700 core.
    fn ipl_step(&mut self, state: IplHle) -> Option<IplHle> {
        match state {
            IplHle::BootDelay { cycles_left } => {
                if cycles_left > 1 {
                    return Some(IplHle::BootDelay {
                        cycles_left: cycles_left - 1,
                    });
                }
                // Boot init "done" — announce. The main CPU spins on
                // $2140/$2141 until it sees $AA/$BB; this is the
                // handshake every upload starts with.
                ipl_trace!("[apu ipl] announcing $AA/$BB, awaiting upload");
                self.memory.port_out[0] = 0xAA;
                self.memory.port_out[1] = 0xBB;
                Some(IplHle::AwaitStart)
            }

            IplHle::AwaitStart => {
                if self.memory.port_in[0] != 0xCC {
                    return Some(state); // keep waiting for the start command
                }
                let addr = u16::from_le_bytes([self.memory.port_in[2], self.memory.port_in[3]]);
                // Ack the start command by echoing $CC.
                self.memory.port_out[0] = 0xCC;

                if self.memory.port_in[1] != 0 {
                    ipl_trace!("[apu ipl] start command: uploading block to {addr:#06x}");
                    Some(IplHle::Transfer { addr, index: 0 })
                } else {
                    ipl_trace!("[apu ipl] start command: direct execute at {addr:#06x}");
                    Some(IplHle::ExecDelay {
                        cycles_left: IPL_EXEC_DELAY_CYCLES,
                        entry: addr,
                    })
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
                    Some(IplHle::Transfer {
                        addr: addr.wrapping_add(1),
                        index: index.wrapping_add(1),
                    })
                } else if delta > 0 {
                    // New command: next block, or execute.
                    let new_addr =
                        u16::from_le_bytes([self.memory.port_in[2], self.memory.port_in[3]]);
                    self.memory.port_out[0] = f4; // ack the command byte

                    if self.memory.port_in[1] != 0 {
                        ipl_trace!("[apu ipl] next block at {new_addr:#06x}");
                        Some(IplHle::Transfer {
                            addr: new_addr,
                            index: 0,
                        })
                    } else {
                        ipl_trace!("[apu ipl] execute at {new_addr:#06x} — handing off shortly");
                        Some(IplHle::ExecDelay {
                            cycles_left: IPL_EXEC_DELAY_CYCLES,
                            entry: new_addr,
                        })
                    }
                } else {
                    // delta < 0: main CPU hasn't advanced yet; keep waiting.
                    Some(state)
                }
            }

            IplHle::ExecDelay { cycles_left, entry } => {
                if cycles_left > 1 {
                    return Some(IplHle::ExecDelay {
                        cycles_left: cycles_left - 1,
                        entry,
                    });
                }
                // Hand off with the real IPL's documented post-jump
                // state: the jump tail stores the entry address at
                // $00/$01 and reaches the jmp with A = X = Y = 0.
                // Uploaded code is entitled to assume all of this.
                let [lo, hi] = entry.to_le_bytes();
                self.memory.ram[0x00] = lo;
                self.memory.ram[0x01] = hi;
                self.cpu.regs.a = 0;
                self.cpu.regs.x = 0;
                self.cpu.regs.y = 0;
                self.cpu.regs.pc = entry;
                ipl_trace!("[apu ipl] SPC700 running at {entry:#06x}");
                None
            }
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
            if let Some(state) = self.ipl {
                // The real chip spends this time executing IPL code from
                // the boot ROM; we run the HLE state machine instead.
                self.ipl = self.ipl_step(state);
            } else if self.cpu.regs.pc >= 0xFFC0 && self.memory.control & 0x80 != 0 {
                // Jumping into $FFC0-$FFFF *while CONTROL bit 7 (IPL ROM
                // enable) is set* re-runs the boot ROM: drivers do this to
                // request another upload (the spc test suite does it
                // between chunks; games do it between tracks). With bit 7
                // clear, $FFC0-$FFFF is ordinary RAM and executes normally
                // — e.g. `pcall $FF` targets $FFFF (spc test #01B2).
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

        // 1. Boot delay, then announce. Before the delay elapses the ports
        // must NOT yet show $AA — that's the point of BootDelay.
        assert_ne!(
            apu.memory.cpu_port_read(0),
            0xAA,
            "no announce before boot delay"
        );
        apu.step(IPL_BOOT_CYCLES as u32 + 8);
        assert_eq!(apu.memory.cpu_port_read(0), 0xAA);
        assert_eq!(apu.memory.cpu_port_read(1), 0xBB);

        // 2. Start command: upload to $0200
        apu.memory.cpu_port_write(2, 0x00);
        apu.memory.cpu_port_write(3, 0x02);
        apu.memory.cpu_port_write(1, 0x01); // non-zero = transfer
        apu.memory.cpu_port_write(0, 0xCC);
        apu.step(2);
        assert_eq!(apu.memory.cpu_port_read(0), 0xCC, "IPL must ack $CC");

        // 3. Upload a 3-byte program: MOV A,#$42 ($E8 $42), then STOP ($FF).
        for (i, byte) in [0xE8_u8, 0x42, 0xFF].iter().enumerate() {
            apu.memory.cpu_port_write(1, *byte);
            apu.memory.cpu_port_write(0, i as u8);
            apu.step(2);
            assert_eq!(apu.memory.cpu_port_read(0), i as u8, "IPL must echo index");
        }
        assert_eq!(&apu.memory.ram[0x0200..0x0203], &[0xE8, 0x42, 0xFF]);

        // 4. Execute command: index jumped by >= 2, port1 = 0, addr = $0200
        apu.memory.cpu_port_write(2, 0x00);
        apu.memory.cpu_port_write(3, 0x02);
        apu.memory.cpu_port_write(1, 0x00); // zero = execute
        apu.memory.cpu_port_write(0, 0x05); // last index was 2; 2 + >=2
        apu.step(2);
        assert_eq!(
            apu.memory.cpu_port_read(0),
            0x05,
            "execute ack must be visible"
        );
        // The ack must stay stable for the whole exec-delay window...
        apu.step(IPL_EXEC_DELAY_CYCLES as u32 - 8);
        assert_eq!(
            apu.memory.cpu_port_read(0),
            0x05,
            "ack stomped during exec delay"
        );
        assert!(apu.ipl_active(), "chunk must not run during exec delay");
        // ...then the uploaded program runs: MOV A,#$42 executes, STOP parks
        // the core. The end-state proves execution began exactly at $0200
        // with the real IPL's zeroed registers.
        apu.step(32);

        assert!(!apu.ipl_active(), "IPL should have handed off");
        assert!(apu.cpu.halted, "uploaded STOP must have parked the core");
        assert_eq!(apu.cpu.regs.a, 0x42, "uploaded MOV A,#$42 must have run");
        assert_eq!(apu.cpu.regs.pc, 0x0203, "PC frozen just past the STOP");
        assert_eq!(
            apu.memory.ram[0x00], 0x00,
            "entry lo stored at $00 like the real IPL"
        );
        assert_eq!(
            apu.memory.ram[0x01], 0x02,
            "entry hi stored at $01 like the real IPL"
        );
        assert_eq!((apu.cpu.regs.x, apu.cpu.regs.y), (0, 0));
    }

    /// Regression test for the chunk-boundary race: a completion value the
    /// previous code left on port 0 must stay readable by the main CPU for
    /// the whole boot delay after an IPL re-entry — not be stomped by the
    /// $AA announce on the next cycle.
    #[test]
    fn test_reentry_preserves_completion_signal_during_boot_delay() {
        let mut apu = Apu::new();
        apu.step(IPL_BOOT_CYCLES as u32 + 8); // initial boot

        // Pretend uploaded code signalled "chunk complete" then jumped
        // back into the boot ROM region.
        apu.memory.port_out[0] = 0x77; // completion signal
        apu.memory.control = 0x80; // IPL ROM mapping enabled
        apu.ipl = None;
        apu.cpu.regs.pc = 0xFFC0;

        // For the entire boot delay the signal must remain visible...
        for _ in 0..(IPL_BOOT_CYCLES as u32 - 2) {
            apu.step(1);
            assert_eq!(
                apu.memory.cpu_port_read(0),
                0x77,
                "signal stomped too early"
            );
        }

        // ...and only then is it replaced by the announce.
        apu.step(8);
        assert_eq!(apu.memory.cpu_port_read(0), 0xAA);
        assert_eq!(apu.memory.cpu_port_read(1), 0xBB);
    }
}

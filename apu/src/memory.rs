use crate::dsp::Dsp;

// ============================================================
// SPC700 MEMORY MAP (relevant I/O region: $00F0–$00FF)
//
// $F0        TEST      — hardware test register (write-only, boot only)
// $F1        CONTROL   — timer enable, ROM/RAM switch, port clear
// $F2        DSPADDR   — DSP register address latch
// $F3        DSPDATA   — DSP register data (read/write via $F2 latch)
// $F4–$F7    CPUIO0–3  — bidirectional communication ports (CPU ↔ APU)
// $F8–$F9    AUXRAM    — extra RAM bytes (readable/writable normally)
// $FA–$FC    TnDIV     — timer 0/1/2 divisor (write-only)
// $FD–$FF    TnOUT     — timer 0/1/2 counter output (read-only, clears on read)
//
// The direct-mapped range $F200–$F27F used by test code is kept alongside
// the real port protocol so both can coexist during development.
// ============================================================

pub struct Memory {
    /// 64 KB APU RAM.  All addresses that are not intercepted as I/O
    /// read/write from/to this array.
    pub ram: [u8; 64 * 1024],

    /// The DSP register file.  Accessed by the CPU exclusively through
    /// the $F2/$F3 address-latch protocol (real hardware) or the direct
    /// $F200–$F27F mapping used by test code.
    pub dsp: Dsp,

    /// $F2 — DSP address latch.
    /// Holds the 7-bit register index for the next $F3 read or write.
    dsp_addr: u8,

    /// $F1 — CONTROL register.
    ///   bit 7: clear port 3 input latch ($F7)
    ///   bit 6: clear port 2 input latch ($F6)
    ///   bit 4: enable timer 2 (64 kHz)
    ///   bit 1: enable timer 1 (8 kHz)
    ///   bit 0: enable timer 0 (8 kHz)
    /// Publicly readable so Timers::step() can inspect the enable bits.
    pub control: u8,

    /// $F4–$F7 — CPU↔APU communication ports.
    ///
    /// The SNES main CPU writes to the APU side of these ports; the SPC700
    /// reads them as inputs.  The SPC700 can also write them; the main CPU
    /// reads those as inputs.  We store both directions separately so that
    /// neither side clobbers the other's data.
    ///
    /// `port_in[n]`  — value written by the SNES CPU, read by SPC700 at $F4+n
    /// `port_out[n]` — value written by SPC700 at $F4+n, read by SNES CPU
    pub port_in:  [u8; 4],
    pub port_out: [u8; 4],

    /// $FA–$FC — Timer divisors (write-only from SPC700 perspective).
    /// Timer N fires every `timer_div[N]` ticks of its base clock.
    /// A divisor of 0 is treated as 256 by hardware.
    pub timer_div: [u8; 3],

    /// $FD–$FF — Timer output counters (read-only, 4-bit, clears on read).
    /// Incremented by the timer hardware; cleared when the SPC700 reads them.
    pub timer_out: [u8; 3],
}

impl Memory {
    pub fn new() -> Self {
        Self {
            ram:       [0u8; 64 * 1024],
            dsp:       Dsp::new(),
            dsp_addr:  0,
            control:   0,
            port_in:   [0u8; 4],
            port_out:  [0u8; 4],
            timer_div: [0u8; 3],
            timer_out: [0u8; 3],
        }
    }

    // ----------------------------------------------------------
    // read8 / read16
    // ----------------------------------------------------------

    pub fn read8(&self, addr: u16) -> u8 {
        match addr {
            // ---- SPC700 I/O ports ($00F0–$00FF) ----

            // $F0 TEST — write-only; reads return 0xFF on hardware but 0 is
            // safe for emulation purposes since no game reads it.
            0x00F0 => 0x00,

            // $F1 CONTROL — write-only on real hardware; return 0.
            0x00F1 => 0x00,

            // $F2 DSPADDR — returns the current address latch value.
            0x00F2 => self.dsp_addr,

            // $F3 DSPDATA — read the DSP register selected by $F2.
            0x00F3 => self.dsp.read_reg(self.dsp_addr),

            // $F4–$F7 CPUIO — SPC700 reads what the SNES CPU wrote.
            0x00F4 => self.port_in[0],
            0x00F5 => self.port_in[1],
            0x00F6 => self.port_in[2],
            0x00F7 => self.port_in[3],

            // $F8–$F9 — auxiliary RAM (normal read).
            0x00F8 | 0x00F9 => self.ram[addr as usize],

            // $FA–$FC — timer divisors are write-only; reads return 0xFF.
            0x00FA..=0x00FC => 0xFF,

            // $FD–$FF — timer output counters.
            // NOTE: on real hardware a read clears the counter.  We cannot
            // do that here because read8 takes &self, not &mut self.
            // The mutable clear is handled by read8_mut below; this path
            // returns the current snapshot for read-only callers (e.g. the
            // debugger).  The CPU must use the &mut path via its memory bus.
            0x00FD => self.timer_out[0],
            0x00FE => self.timer_out[1],
            0x00FF => self.timer_out[2],

            // ---- Direct-mapped DSP register window ($F200–$F27F) ----
            // Kept for test code that constructs Memory directly and writes
            // DSP registers without going through the $F2/$F3 protocol.
            0xF200..=0xF27F => self.dsp.read_reg((addr - 0xF200) as u8),

            // ---- Normal RAM ----
            _ => self.ram[addr as usize],
        }
    }

    /// Like read8 but takes &mut self so that reading $FD–$FF can clear
    /// the timer output counters, matching real hardware behaviour.
    /// The SPC700 CPU must call this variant when executing load instructions.
    pub fn read8_mut(&mut self, addr: u16) -> u8 {
        match addr {
            0x00FD => { let v = self.timer_out[0]; self.timer_out[0] = 0; v }
            0x00FE => { let v = self.timer_out[1]; self.timer_out[1] = 0; v }
            0x00FF => { let v = self.timer_out[2]; self.timer_out[2] = 0; v }
            _      => self.read8(addr),
        }
    }

    pub fn read16(&self, addr: u16) -> u16 {
        let lo = self.read8(addr) as u16;
        let hi = self.read8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    // ----------------------------------------------------------
    // write8 / write16
    // ----------------------------------------------------------

    pub fn write8(&mut self, addr: u16, val: u8) {
        match addr {
            // $F0 TEST — only relevant during hardware boot; ignore safely.
            0x00F0 => {}

            // $F1 CONTROL
            // bit 7: clear port 3 ($F7) input latch
            // bit 6: clear port 2 ($F6) input latch
            // bits 4/1/0: timer enables (forwarded to Timers via the register)
            0x00F1 => {
                self.control = val;
                if val & 0x80 != 0 { self.port_in[3] = 0; }
                if val & 0x40 != 0 { self.port_in[2] = 0; }
                // Timer enable bits are read by Timers::step() via memory.control.
            }

            // $F2 DSPADDR — latch the register index for the next $F3 access.
            // Only 7 bits are meaningful; the high bit is masked by the DSP.
            0x00F2 => self.dsp_addr = val & 0x7F,

            // $F3 DSPDATA — write to the DSP register selected by $F2.
            0x00F3 => self.dsp.write_reg(self.dsp_addr, val),

            // $F4–$F7 CPUIO — SPC700 writes; SNES CPU reads these.
            0x00F4 => self.port_out[0] = val,
            0x00F5 => self.port_out[1] = val,
            0x00F6 => self.port_out[2] = val,
            0x00F7 => self.port_out[3] = val,

            // $F8–$F9 — auxiliary RAM.
            0x00F8 | 0x00F9 => self.ram[addr as usize] = val,

            // $FA–$FC — timer divisors.
            0x00FA => self.timer_div[0] = val,
            0x00FB => self.timer_div[1] = val,
            0x00FC => self.timer_div[2] = val,

            // $FD–$FF — read-only timer counters; writes are ignored.
            0x00FD..=0x00FF => {}

            // ---- Direct-mapped DSP register window ($F200–$F27F) ----
            // Kept for test code; real SPC700 programs use $F2/$F3 above.
            0xF200..=0xF27F => self.dsp.write_reg((addr - 0xF200) as u8, val),

            // ---- Normal RAM ----
            _ => self.ram[addr as usize] = val,
        }
    }

    pub fn write16(&mut self, addr: u16, value: u16) {
        self.write8(addr,                      (value & 0xFF) as u8);
        self.write8(addr.wrapping_add(1), (value >> 8) as u8);
    }

    // ----------------------------------------------------------
    // SNES CPU ↔ APU communication helpers
    //
    // The SNES main CPU accesses these from its side of the bus.
    // These are called by the top-level emulator, not by the SPC700.
    // ----------------------------------------------------------

    /// Write a value to communication port `n` (0–3) from the SNES CPU side.
    /// The SPC700 will read this via $F4+n.
    pub fn cpu_port_write(&mut self, port: usize, val: u8) {
        if port < 4 {
            self.port_in[port] = val;
        }
    }

    /// Read the value the SPC700 wrote to communication port `n` (0–3).
    /// The SNES CPU reads this to receive data from the APU.
    pub fn cpu_port_read(&self, port: usize) -> u8 {
        if port < 4 { self.port_out[port] } else { 0 }
    }
}

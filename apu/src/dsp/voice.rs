use crate::memory::RawARAM;

use super::adsr::{Adsr, EnvelopePhase};
use super::brr::{Brr, decode_brr_block, ram_read8};

/// One voice (channel) of the SNES APU DSP.
#[derive(Debug, Clone, Copy, Default)]
pub struct Voice {
    /// Left channel volume, signed (-128..+127).
    pub left_vol: i8,

    /// Right channel volume, signed (-128..+127).
    pub right_vol: i8,

    /// 14-bit pitch value (0x0000–0x3FFF).
    /// 0x1000 = playback at the native 32 kHz sample rate.
    pub pitch: u16,

    /// Sample source number: index into the DIR table in APU RAM.
    pub srcn: u8,

    /// Whether this voice is currently keyed on (actively playing).
    pub key_on: bool,

    /// 16-bit pitch counter used to pace sample consumption.
    /// Every 0x1000 units = 1 BRR sample consumed.
    pub pitch_counter: u16,

    /// Most recently output sample (16-bit, pre-envelope).
    pub current_sample: i16,

    /// ADSR envelope sub-state.
    pub adsr: Adsr,

    /// BRR decoder sub-state.
    pub brr: Brr,
}

impl Voice {
    /// Advance this voice by one DSP tick.
    ///
    /// `i` is the voice index (0–7), used to compute ENVX/OUTX register
    /// offsets and the ENDX bitmask.
    /// `registers` is the DSP register file; ENVX, OUTX, and ENDX are
    /// written here so the CPU can read them back via `$F3`.
    pub fn step(&mut self, i: usize, ram: &RawARAM, registers: &mut [u8; 128]) {
        // 1. Envelope update
        if self.adsr.envelope_phase != EnvelopePhase::Off {
            self.adsr.update_envelope();
        }

        if !self.key_on && self.adsr.envelope_phase == EnvelopePhase::Off {
            return;
        }
        if !self.key_on {
            return;
        }

        // 2. Resolve DIR table on first tick after key-on.
        // buffer_fill == 0 means no block has been decoded yet.
        if self.brr.buffer_fill == 0 {
            let dir_entry = self.brr.addr;

            let start_lo = ram_read8(ram, dir_entry)     as u16;
            let start_hi = ram_read8(ram, dir_entry + 1) as u16;
            let loop_lo  = ram_read8(ram, dir_entry + 2) as u16;
            let loop_hi  = ram_read8(ram, dir_entry + 3) as u16;

            self.brr.addr      = (start_hi << 8) | start_lo;
            self.brr.loop_addr = (loop_hi  << 8) | loop_lo;

            self.decode_next_block(i, ram, registers);
        }

        // 3. Pitch counter advance.
        // Every 0x1000 units = one BRR sample consumed.
        let pitch = self.pitch & 0x3FFF;
        self.pitch_counter = self.pitch_counter.wrapping_add(pitch);

        let samples_to_consume = self.pitch_counter / 0x1000;
        self.pitch_counter %= 0x1000;

        // 4. Consume decoded samples from buffer.
        for _ in 0..samples_to_consume {
            let idx = self.brr.nibble_idx as usize;
            if idx < self.brr.buffer_fill as usize {
                self.current_sample = self.brr.sample_buffer[idx];
                self.brr.nibble_idx += 1;
            }

            if self.brr.nibble_idx >= self.brr.buffer_fill {
                self.brr.nibble_idx = 0;
                self.decode_next_block(i, ram, registers);
                if !self.key_on {
                    break;
                }
            }
        }

        // 5. Update read-only ENVX ($X8) and OUTX ($X9) registers.
        //   ENVX = envelope_level >> 4  (11-bit → 7-bit)
        //   OUTX = current_sample  >> 8 (signed top byte)
        registers[(i << 4) | 0x8] = (self.adsr.envelope_level >> 4) as u8;
        registers[(i << 4) | 0x9] = (self.current_sample >> 8) as u8;
    }

    /// Decode the next 9-byte BRR block and advance the BRR address.
    ///
    /// Handles end/loop flags:
    /// - end=true,  loop=true  → jump to loop_addr and continue
    /// - end=true,  loop=false → silence the voice (enter release)
    /// - end=false             → advance address by 9 bytes
    ///
    /// Sets bit `i` of `registers[0x7C]` (ENDX) when an end block is reached.
    fn decode_next_block(&mut self, i: usize, ram: &RawARAM, registers: &mut [u8; 128]) {
        let (samples, end, do_loop) = decode_brr_block(
            ram,
            self.brr.addr,
            &mut self.brr.prev1,
            &mut self.brr.prev2,
        );

        self.brr.sample_buffer = samples;
        self.brr.buffer_fill   = 16;
        self.brr.nibble_idx    = 0;

        if end {
            registers[0x7C] |= 1u8 << i;

            if do_loop {
                self.brr.addr = self.brr.loop_addr;
            } else {
                self.key_on = false;
                self.adsr.envelope_phase = EnvelopePhase::Release;
            }
        } else {
            self.brr.addr = self.brr.addr.wrapping_add(9);
        }
    }
}

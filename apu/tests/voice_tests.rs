use apu::Memory;
/// Voice and per-voice register mapping tests
///
/// Covers Voice and Brr default state, all per-voice DSP register
/// mappings (VOL, PITCH, SRCN, ADSR1, ADSR2), and independence
/// across all 8 voices.
use apu::dsp::{Brr, EnvelopePhase, Voice};

// ============================================================
// Helpers
// ============================================================

const DSP_BASE: u16 = 0xF200;

fn dsp_vw(mem: &mut Memory, voice: u8, reg: u8, val: u8) {
    mem.write8(DSP_BASE + ((voice as u16) << 4) + reg as u16, val);
}

// ============================================================
// Voice — default state
// ============================================================

#[test]
fn test_voice_default() {
    let v = Voice::default();
    assert_eq!(v.left_vol, 0);
    assert_eq!(v.right_vol, 0);
    assert_eq!(v.pitch, 0);
    assert_eq!(v.srcn, 0);
    assert!(!v.key_on);
    assert_eq!(v.pitch_counter, 0);
    assert_eq!(v.current_sample, 0);
    assert_eq!(v.adsr.envelope_phase, EnvelopePhase::Off);
    assert_eq!(v.adsr.envelope_level, 0);
    assert_eq!(v.brr.addr, 0);
    assert_eq!(v.brr.nibble_idx, 0);
    assert_eq!(v.brr.buffer_fill, 0);
}

#[test]
fn test_brr_default_all_zero() {
    let brr = Brr::default();
    assert_eq!(brr.addr, 0, "addr must be 0");
    assert_eq!(brr.nibble_idx, 0, "nibble_idx must be 0");
    assert_eq!(brr.prev1, 0, "prev1 must be 0");
    assert_eq!(brr.prev2, 0, "prev2 must be 0");
    assert_eq!(brr.loop_addr, 0, "loop_addr must be 0");
    assert_eq!(
        brr.buffer_fill, 0,
        "buffer_fill must be 0 (no block decoded)"
    );
    assert_eq!(
        brr.sample_buffer, [0i16; 16],
        "sample_buffer must be all-zero"
    );
}

// ============================================================
// Dsp — per-voice register mapping (voice N at index N*0x10)
// ============================================================

#[test]
fn test_vol_left_maps_to_correct_voice() {
    let mut mem = Memory::new();
    // Voice 0: reg 0x00, Voice 3: reg 0x30, Voice 7: reg 0x70
    dsp_vw(&mut mem, 0, 0x0, 0x10);
    dsp_vw(&mut mem, 3, 0x0, 0x30);
    dsp_vw(&mut mem, 7, 0x0, 0x70);
    assert_eq!(mem.dsp.voices[0].left_vol, 0x10i8);
    assert_eq!(mem.dsp.voices[3].left_vol, 0x30i8);
    assert_eq!(mem.dsp.voices[7].left_vol, 0x70i8);
}

#[test]
fn test_vol_right_maps_to_correct_voice() {
    let mut mem = Memory::new();
    dsp_vw(&mut mem, 0, 0x1, 0x55);
    dsp_vw(&mut mem, 5, 0x1, 0x22);
    assert_eq!(mem.dsp.voices[0].right_vol, 0x55u8 as i8);
    assert_eq!(mem.dsp.voices[5].right_vol, 0x22u8 as i8);
}

#[test]
fn test_pitch_low_high_bytes_combine_correctly() {
    let mut mem = Memory::new();
    // Voice 1: PITCH low = 0xAB, PITCH high = 0x3C (only 6 bits = 0x3C & 0x3F = 0x3C)
    dsp_vw(&mut mem, 1, 0x2, 0xAB); // low
    dsp_vw(&mut mem, 1, 0x3, 0x3C); // high (14-bit → bits 13-8 = 0x3C & 0x3F)
    let expected: u16 = ((0x3C_u16 & 0x3F) << 8) | 0xAB;
    assert_eq!(mem.dsp.voices[1].pitch, expected);
}

#[test]
fn test_pitch_clamped_to_14_bits() {
    let mut mem = Memory::new();
    // Write 0xFF to high byte; only low 6 bits should survive.
    dsp_vw(&mut mem, 0, 0x3, 0xFF);
    assert_eq!(
        mem.dsp.voices[0].pitch & !0x3FFF,
        0,
        "bits above 13 must be zero"
    );
}

#[test]
fn test_srcn_register_written_correctly() {
    let mut mem = Memory::new();
    dsp_vw(&mut mem, 2, 0x4, 0x1F);
    assert_eq!(mem.dsp.voices[2].srcn, 0x1F);
}

#[test]
fn test_adsr1_register_layout() {
    // ADSR1 = EDDDAAAA
    //   bit 7   = adsr_mode
    //   bits 6-4 = decay_rate
    //   bits 3-0 = attack_rate
    let mut mem = Memory::new();
    // 0b1_011_0101 = 0xB5 → mode=1, decay=3, attack=5
    dsp_vw(&mut mem, 0, 0x5, 0xB5);
    assert!(mem.dsp.voices[0].adsr.adsr_mode);
    assert_eq!(mem.dsp.voices[0].adsr.decay_rate, 0x03);
    assert_eq!(mem.dsp.voices[0].adsr.attack_rate, 0x05);
}

#[test]
fn test_adsr1_mode_bit_zero() {
    let mut mem = Memory::new();
    dsp_vw(&mut mem, 0, 0x5, 0x35); // bit 7 = 0
    assert!(!mem.dsp.voices[0].adsr.adsr_mode);
}

#[test]
fn test_adsr2_register_layout() {
    // ADSR2 = SSSRRRRR
    //   bits 7-5 = sustain_level
    //   bits 4-0 = sustain_rate
    let mut mem = Memory::new();
    // 0b101_10110 = 0xB6 → level=5, rate=22
    dsp_vw(&mut mem, 0, 0x6, 0xB6);
    assert_eq!(mem.dsp.voices[0].adsr.sustain_level, 5);
    assert_eq!(mem.dsp.voices[0].adsr.sustain_rate, 22);
}

#[test]
fn test_all_8_voices_have_independent_registers() {
    let mut mem = Memory::new();
    for v in 0u8..8 {
        dsp_vw(&mut mem, v, 0x0, v * 10); // left vol
        dsp_vw(&mut mem, v, 0x4, v); // srcn
    }
    for v in 0..8usize {
        assert_eq!(mem.dsp.voices[v].left_vol, (v as u8 * 10) as i8);
        assert_eq!(mem.dsp.voices[v].srcn, v as u8);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// A single BRR block, shift=12 filter=0 (each nibble decodes
    /// independently of history), that loops to itself forever. Nibbles
    /// alternate +7/-8, so decoded samples strictly alternate between a
    /// large positive and large negative value — an easy signal to check
    /// that playback is actually advancing sample-by-sample.
    fn build_test_ram() -> Box<RawARAM> {
        let mut ram: Box<RawARAM> = Box::new([0u8; 64 * 1024]);
        // DIR entry at $0010: start=$0020, loop=$0020 (loops to itself).
        ram[0x0010] = 0x20;
        ram[0x0011] = 0x00;
        ram[0x0012] = 0x20;
        ram[0x0013] = 0x00;
        // BRR header: shift=12 ($C), filter=0, loop=1, end=1 -> 0xC3.
        ram[0x0020] = 0xC3;
        for i in 0..8 {
            ram[0x0021 + i] = 0x78; // nibbles 7, -8 repeating
        }
        ram
    }

    /// Regression test for the frozen-release bug: once KOFF clears
    /// `key_on`, the voice must keep decoding/consuming BRR samples
    /// under its fading envelope instead of freezing on whatever sample
    /// happened to be current at the moment of key-off.
    #[test]
    fn release_keeps_consuming_samples_instead_of_freezing() {
        let ram = build_test_ram();
        let mut registers = [0u8; 128];
        let mut voice = Voice {
            key_on: true,
            pitch: 0x1000, // exactly one decoded sample consumed per tick
            ..Default::default()
        };
        voice.brr.addr = 0x0010; // DIR entry address, as key_on_voice sets it
        voice.adsr.adsr_mode = true;
        voice.adsr.attack_rate = 15; // instant attack, out of the way
        voice.adsr.envelope_phase = EnvelopePhase::Attack;

        // First tick: resolves the DIR entry, decodes the block, and
        // consumes sample_buffer[0].
        voice.step(0, &ram, &mut registers);

        // Simulate KOFF exactly as Dsp::write_reg($5C) does.
        voice.key_on = false;
        voice.adsr.envelope_phase = EnvelopePhase::Release;

        let mut samples = Vec::new();
        for _ in 0..4 {
            voice.step(0, &ram, &mut registers);
            samples.push(voice.current_sample);
        }

        // The block's decoded samples strictly alternate sign, so
        // continued playback must alternate too. The old code returned
        // immediately once key_on was false, leaving current_sample
        // pinned at its pre-KOFF value for every one of these calls.
        assert_ne!(samples[0], samples[1], "sample must advance during release, not freeze");
        assert_ne!(samples[1], samples[2], "sample must advance during release, not freeze");
        assert_ne!(samples[2], samples[3], "sample must advance during release, not freeze");
    }

    /// A voice that never keys on, and one whose release has fully
    /// finished (envelope reached Off), must both stay idle — the fix
    /// should only unfreeze the *active* Release window, not resurrect
    /// voices that are genuinely done.
    #[test]
    fn fully_off_voice_stays_idle() {
        let ram = build_test_ram();
        let mut registers = [0u8; 128];
        let mut voice = Voice::default(); // key_on=false, phase=Off

        voice.step(0, &ram, &mut registers);

        assert_eq!(voice.current_sample, 0, "untouched voice must stay silent");
        assert_eq!(voice.brr.buffer_fill, 0, "must never resolve DIR/decode for an idle voice");
    }
}

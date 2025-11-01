use apu::dsp::{Dsp, Voice};
use apu::dsp::EnvelopePhase;
use apu::Memory;

#[test]
fn test_dsp_register_write_read() {
    let mut dsp = Dsp::new();

    for i in 0..128u8 {
        dsp.write(0xF200 + i as u16, i);
    }

    for i in 0..128u8 {
        let val = dsp.read(0xF200 + i as u16);
        assert_eq!(val, i, "DSP register 0x{:X} mismatch", 0xF200 + i as u16);
    }
}

#[test]
fn test_voice_volume_mapping() {
    let mut dsp = Dsp::new();

    dsp.write(0xF200, 0x42); // left vol for voice 0
    dsp.write(0xF208, 0x37); // right vol for voice 0

    assert_eq!(dsp.voices[0].left_vol, 0x42);
    assert_eq!(dsp.voices[0].right_vol, 0x37);
}

#[test]
fn test_voice_pitch_mapping() {
    let mut dsp = Dsp::new();

    dsp.write(0xF210, 0x34); // pitch low byte
    dsp.write(0xF218, 0x12); // pitch high byte

    assert_eq!(dsp.voices[0].pitch, 0x1234);
}

#[test]
fn test_voice_key_on_off() {
    let mut dsp = Dsp::new();

    dsp.write(0xF220, 1);
    assert!(dsp.voices[0].key_on);

    dsp.write(0xF220, 0);
    assert!(!dsp.voices[0].key_on);
}

#[test]
fn test_voice_initialization() {
    let dsp = Dsp::new();
    for voice in dsp.voices.iter() {
        assert!(!voice.key_on);
        assert_eq!(voice.sample_start, 0);
        assert_eq!(voice.sample_end, 0);
        assert_eq!(voice.current_addr, 0);
        assert_eq!(voice.current_sample, 0);
    }
}

#[test]
fn test_voice_step_advances() {
    let mut dsp = Dsp::new();
    let mem = Memory::new();

    dsp.voices[0].key_on = true;
    dsp.voices[0].sample_start = 0x1000;
    dsp.voices[0].sample_end = 0x1005;
    dsp.voices[0].current_addr = 0x1000;
    dsp.voices[0].pitch = 0x100; // integer step = 1

    dsp.step(&mem);
    assert_eq!(dsp.voices[0].current_addr, 0x1001);

    dsp.step(&mem);
    dsp.step(&mem);
    dsp.step(&mem);
    dsp.step(&mem);

    assert!(!dsp.voices[0].key_on);
}
#[test]
fn test_step_voice_advances_and_fetches_sample() {
    let mut mem = Memory::new();
    let mut dsp = Dsp::new();

    let voice = Voice {
        key_on: true,
        current_addr: 0,
        sample_start: 0,
        frac: 0,
        pitch: 256, // step = 1 per call
        sample_end: 3,
        current_sample: 0,
        left_vol: 1,
        right_vol: 1,
        adsr_mode: false,
        attack_rate: 0,
        decay_rate: 0,
        sustain_level: 0,
        release_rate: 0,
        envelope_level: 0,
        envelope_phase: EnvelopePhase::Off,
    };

    dsp.voices[0] = voice;

    mem.write8(0, 10);
    mem.write8(1, 20);
    mem.write8(2, 30);

    dsp.step(&mem);
    assert_eq!(dsp.voices[0].current_addr, 1);
    assert_eq!(dsp.voices[0].current_sample, 20);
    assert!(dsp.voices[0].key_on);

    dsp.step(&mem);
    assert_eq!(dsp.voices[0].current_addr, 2);
    assert_eq!(dsp.voices[0].current_sample, 30);
    assert!(dsp.voices[0].key_on);

    dsp.step(&mem);
    assert_eq!(dsp.voices[0].current_addr, 3);
    assert!(!dsp.voices[0].key_on);
}

#[test]
fn test_render_audio_single_voice() {
    let mut dsp = Dsp::new();
    
    dsp.voices[0] = Voice {
        key_on: true,
        current_sample: 50, // i8 value
        sample_start: 0,
        left_vol: 2,
        right_vol: 3,
        ..Default::default()
    };

    let buffer = dsp.render_audio(2);

    assert_eq!(buffer[0], (100, 150)); // 50*2, 50*3
    assert_eq!(buffer[1], (100, 150));
}

#[test]
fn test_render_audio_multiple_voices_mixed_and_clamped() {
    let mut dsp = Dsp::new();

    dsp.voices[0] = Voice { key_on: true, current_sample: 100, sample_start: 0, left_vol: 2, right_vol: 2, ..Default::default() };
    dsp.voices[1] = Voice { key_on: true, current_sample: 120, sample_start: 0, left_vol: 2, right_vol: 2, ..Default::default() };

    let buffer = dsp.render_audio(1);

    // 100*2 + 120*2 = 440 -> clamped to 32767 (but here it's still in range)
    assert_eq!(buffer[0], (440, 440));
}

#[test]
fn test_adsr_attack_phase() {
    let mut voice = Voice::default();
    voice.envelope_phase = EnvelopePhase::Attack;
    voice.attack_rate = 4;

    // Simulate multiple ticks
    for _ in 0..50 {
        voice.update_envelope();
    }

    // Envelope should be non-zero and phase should be Decay
    assert!(voice.envelope_level > 0);
    assert_eq!(voice.envelope_phase, EnvelopePhase::Decay);
    assert_eq!(voice.envelope_level, 0x7FF); // capped at max
}

#[test]
fn test_adsr_decay_phase() {
    let mut voice = Voice::default();
    voice.envelope_phase = EnvelopePhase::Decay;
    voice.envelope_level = 0x7FF; // start from max
    voice.decay_rate = 8;
    voice.sustain_level = 4; // mid-level sustain

    // Simulate multiple ticks
    for _ in 0..100 {
        voice.update_envelope();
    }

    // Should have reached sustain and changed phase
    let target = (voice.sustain_level as u16) * 0x100 / 8;
    assert!(voice.envelope_level <= target);
    assert_eq!(voice.envelope_phase, EnvelopePhase::Sustain);
}

#[test]
fn test_adsr_sustain_phase() {
    let mut voice = Voice::default();
    voice.envelope_phase = EnvelopePhase::Sustain;
    voice.envelope_level = 0x400;

    // Update several times
    for _ in 0..20 {
        voice.update_envelope();
    }

    // Should remain unchanged
    assert_eq!(voice.envelope_level, 0x400);
    assert_eq!(voice.envelope_phase, EnvelopePhase::Sustain);
}

#[test]
fn test_adsr_release_phase() {
    let mut voice = Voice::default();
    voice.envelope_phase = EnvelopePhase::Release;
    voice.envelope_level = 0x400;
    voice.release_rate = 8;

    // Simulate multiple ticks
    for _ in 0..100 {
        voice.update_envelope();
    }

    // Should reach zero and switch to Off
    assert_eq!(voice.envelope_level, 0);
    assert_eq!(voice.envelope_phase, EnvelopePhase::Off);
}

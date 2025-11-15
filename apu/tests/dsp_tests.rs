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

    let voice = &mut dsp.voices[0];

    voice.left_vol = 100;
    voice.right_vol = 150;

    voice.current_sample = 30; // raw sample
    voice.envelope_level = 0x7FF; // full volume
    voice.envelope_phase = EnvelopePhase::Sustain;

    let mut out = [(0_i16, 0_i16); 1];

    dsp.render_audio(&mut out);

    let expected_left  = (30 * 100 / 127) as i16;
    let expected_right = (30 * 150 / 127) as i16;

    assert_eq!(out[0], (expected_left, expected_right));
}

#[test]
fn test_render_audio_multiple_voices_mixed_and_clamped() {
    let mut dsp = Dsp::new();

    // Voice 1
    dsp.voices[0].left_vol = 127;
    dsp.voices[0].right_vol = 127;
    dsp.voices[0].current_sample = 100;
    dsp.voices[0].envelope_level = 0x7FF;
    dsp.voices[0].envelope_phase = EnvelopePhase::Sustain;

    // Voice 2 (same values → doubles output)
    dsp.voices[1].left_vol = 127;
    dsp.voices[1].right_vol = 127;
    dsp.voices[1].current_sample = 100;
    dsp.voices[1].envelope_level = 0x7FF;
    dsp.voices[1].envelope_phase = EnvelopePhase::Sustain;

    let mut out = [(0_i16, 0_i16); 1];

    dsp.render_audio(&mut out);

    let expected = (100 * 2).clamp(i16::MIN as i32, i16::MAX as i32) as i16;

    assert_eq!(out[0], (expected, expected));
}

#[test]
fn test_adsr_attack_phase() {
    let mut v = Voice::default();
    v.attack_rate = 10;
    v.envelope_phase = EnvelopePhase::Attack;

    for _ in 0..30 {
        let prev = v.envelope_level;
        v.update_envelope();

        if prev < 0x7FF {
            // The ONLY valid phases during attack ramp-up are: Attack OR Decay (if we hit max)
            assert!(
                matches!(v.envelope_phase, EnvelopePhase::Attack | EnvelopePhase::Decay),
                "Phase can only be Attack or transition to Decay when hitting max"
            );
            // Once we switch to Decay, we should stop testing Attack progression
            if v.envelope_phase == EnvelopePhase::Decay {
                assert_eq!(v.envelope_level, 0x7FF, "Decay must start exactly at max envelope");
                return; // test passes
            }
        }
    }
    panic!("Attack never reached Decay within expected iterations");
}

#[test]
fn test_adsr_decay_phase() {
    let mut voice = Voice::default();
    voice.envelope_phase = EnvelopePhase::Decay;

    voice.decay_rate = 5;
    voice.sustain_level = 8; // target = 256

    voice.envelope_level = 700; // above sustain threshold

    for _ in 0..50 {
        voice.update_envelope();
        if voice.envelope_phase == EnvelopePhase::Sustain {
            break;
        }
    }

    assert_eq!(voice.envelope_phase, EnvelopePhase::Sustain);
    assert!(voice.envelope_level <= 256);
}

#[test]
fn test_adsr_sustain_phase() {
    let mut voice = Voice::default();
    voice.envelope_phase = EnvelopePhase::Sustain;
    voice.envelope_level = 500;

    for _ in 0..10 {
        voice.update_envelope();
    }

    assert_eq!(voice.envelope_phase, EnvelopePhase::Sustain);
    assert_eq!(voice.envelope_level, 500);
}

#[test]
fn test_adsr_release_phase() {
    let mut voice = Voice::default();
    voice.envelope_phase = EnvelopePhase::Release;

    voice.release_rate = 8;
    voice.envelope_level = 300;

    for _ in 0..50 {
        voice.update_envelope();
        if voice.envelope_phase == EnvelopePhase::Off {
            break;
        }
    }

    assert_eq!(voice.envelope_phase, EnvelopePhase::Off);
    assert_eq!(voice.envelope_level, 0);
}

#[test]
fn test_adsr_full_cycle() {
    let mut v = Voice::default();
    v.attack_rate = 20;
    v.decay_rate = 4;
    v.sustain_level = 6;
    v.release_rate = 8;

    v.envelope_phase = EnvelopePhase::Attack;

    // Attack → Decay
    while v.envelope_phase == EnvelopePhase::Attack {
        v.update_envelope();
    }
    assert_eq!(v.envelope_phase, EnvelopePhase::Decay);

    // Calculate correct sustain threshold
    let sustain_target = (v.sustain_level as u16) * 0x100 / 8;

    // Decay → Sustain
    while v.envelope_phase == EnvelopePhase::Decay {
        v.update_envelope();
    }

    assert_eq!(v.envelope_phase, EnvelopePhase::Sustain);

    // Envelope should be <= sustain target (not necessarily equal)
    assert!(
        v.envelope_level <= sustain_target,
        "Envelope must be at or below computed sustain target"
    );

    // Release → Off
    v.envelope_phase = EnvelopePhase::Release;

    while v.envelope_phase == EnvelopePhase::Release {
        v.update_envelope();
    }

    assert_eq!(v.envelope_phase, EnvelopePhase::Off);
    assert_eq!(v.envelope_level, 0);
}



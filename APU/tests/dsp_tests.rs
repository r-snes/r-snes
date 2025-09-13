use apu::dsp::Dsp;
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
fn test_voice_sample_fetch() {
    let mut dsp = Dsp::new();
    let mut mem = Memory::new();

    // Sample data at 0x1000: 0x00, 0x80, 0xFF
    mem.write8(0x1000, 0x00);
    mem.write8(0x1001, 0x80);
    mem.write8(0x1002, 0xFF);

    // Activate voice
    dsp.voices[0].key_on = true;
    dsp.voices[0].sample_start = 0x1000;
    dsp.voices[0].sample_end = 0x1003;
    dsp.voices[0].current_addr = 0x1000;
    dsp.voices[0].pitch = 0x100; // integer increment = 1

    // Step once: fetch first sample
    dsp.step(&mem);
    assert_eq!(dsp.voices[0].current_sample, 0x00i8); // 0x00 -> 0

    // Step twice: fetch second sample
    dsp.step(&mem);
    assert_eq!(dsp.voices[0].current_sample, -128i8); // 0x80 -> -128

    // Step thrice: fetch third sample
    dsp.step(&mem);
    assert_eq!(dsp.voices[0].current_sample, -1i8); // 0xFF -> -1
}

#[test]
fn test_voice_sample_mixing() {
    let mut dsp = Dsp::new();
    let mut mem = Memory::new();

    // Sample values
    mem.write8(0x1000, 0x10); // 16
    mem.write8(0x1001, 0x20); // 32

    // Configure voice with different left/right volumes
    dsp.voices[0].key_on = true;
    dsp.voices[0].sample_start = 0x1000;
    dsp.voices[0].sample_end = 0x1002;
    dsp.voices[0].current_addr = 0x1000;
    dsp.voices[0].pitch = 1;
    dsp.voices[0].left_vol = 2;
    dsp.voices[0].right_vol = 4;

    // Step once to fetch first sample
    dsp.step(&mem);
    let mix = dsp.render_audio(1);
    assert_eq!(mix[0].0, 16 * 2); // left
    assert_eq!(mix[0].1, 16 * 4); // right

    // Step again to fetch second sample
    dsp.step(&mem);
    let mix = dsp.render_audio(1);
    assert_eq!(mix[0].0, 32 * 2); // left
    assert_eq!(mix[0].1, 32 * 4); // right
}

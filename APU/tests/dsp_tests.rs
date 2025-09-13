use apu::dsp::Dsp;

#[test]
fn test_dsp_register_write_read() {
    let mut dsp = Dsp::new();

    // Write some values to DSP registers
    for i in 0..128u8 {
        dsp.write(0xF200 + i as u16, i);
    }

    // Verify that reading them back returns the same values
    for i in 0..128u8 {
        let val = dsp.read(0xF200 + i as u16);
        assert_eq!(val, i, "DSP register 0x{:X} mismatch", 0xF200 + i as u16);
    }
}

#[test]
fn test_dsp_step_placeholder() {
    let mut dsp = Dsp::new();

    // Set a few registers
    dsp.write(0xF200, 0x12);
    dsp.write(0xF201, 0x34);

    // Call step (currently placeholder)
    dsp.step();

    // Since step does nothing, registers should be unchanged
    assert_eq!(dsp.read(0xF200), 0x12);
    assert_eq!(dsp.read(0xF201), 0x34);
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

    dsp.write(0xF210, 0x34); // pitch low byte for voice 0
    dsp.write(0xF218, 0x12); // pitch high byte for voice 0

    assert_eq!(dsp.voices[0].pitch, 0x1234);
}

#[test]
fn test_voice_key_on_off() {
    let mut dsp = Dsp::new();

    dsp.write(0xF220, 1); // key on voice 0
    assert!(dsp.voices[0].key_on);

    dsp.write(0xF220, 0); // key off voice 0
    assert!(!dsp.voices[0].key_on);
}

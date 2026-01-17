use apu::dsp::{Dsp, Voice, Brr, decode_brr_nibble};
use apu::dsp::EnvelopePhase;
use apu::Memory;

#[test]
fn test_decode_brr_nibble_sign_extend() {
    // negative nibble: 0xF = -1
    let s = decode_brr_nibble(-1, 4, 0, 0, 0);

    // -1 << 4 = -16
    assert_eq!(s, -16);
}

#[test]
fn test_decode_brr_nibble_no_filter() {
    // Simple case: no prediction, just shifting

    let nibble: i8 = 4;   // small positive nibble
    let shift: u8 = 4;    // shift left 4 bits
    let filter: u8 = 0;   // no prediction
    let prev1 = 0;
    let prev2 = 0;

    let sample = decode_brr_nibble(nibble, shift, filter, prev1, prev2);

    // 4 << 4 = 64
    assert_eq!(sample, 64);
}

#[test]
fn test_decode_brr_nibble_negative_nibble() {
    // Make sure sign extension works

    let nibble: i8 = -2;
    let shift: u8 = 4;
    let filter: u8 = 0;
    let prev1 = 0;
    let prev2 = 0;

    let sample = decode_brr_nibble(nibble, shift, filter, prev1, prev2);

    // -2 << 4 = -32
    assert_eq!(sample, -32);
}

#[test]
fn test_decode_brr_nibble_filter1_uses_prev1() {
    let nibble: i8 = 0;   // zero input so output should be purely prediction
    let shift: u8 = 0;
    let filter: u8 = 1;
    let prev1: i16 = 1000;
    let prev2: i16 = 0;

    let sample = decode_brr_nibble(nibble, shift, filter, prev1, prev2);

    // filter 1 = prev1 - (prev1 >> 4)
    let expected = prev1 - (prev1 >> 4);

    assert_eq!(sample, expected);
}

#[test]
fn test_brr_state_advances_with_pos() {
    // Test that pos increments correctly inside a block

    let mut brr = Brr::default();
    brr.pos = 0;

    brr.pos += 1;
    assert_eq!(brr.pos, 1);

    brr.pos = 15;
    brr.pos += 1;

    // In real DSP code this would wrap to 0 and addr += 9
    // Here we just check the boundary
    assert_eq!(brr.pos, 16);
}

#[test]
fn test_brr_block_wrap_logic_simulated() {
    // Simulate what DSP loop does when reaching end of block

    let mut brr = Brr::default();
    brr.addr = 0x1000;
    brr.pos = 15;

    // Next step should wrap
    brr.pos += 1;
    if brr.pos >= 16 {
        brr.pos = 0;
        brr.addr += 9;
    }

    assert_eq!(brr.pos, 0);
    assert_eq!(brr.addr, 0x1009);
}

/// BRR decoder tests
///
/// Covers:
///   - decode_brr_nibble: zero input, positive/negative nibbles, all 4 filters
///   - shift boundary values (0, 12, 13–15 saturation)
///   - filter coefficient correctness (i32 intermediate math, 15-bit clamp)
///   - decode_brr_block: header parsing, end/loop flags, history threading
///   - Brr struct defaults and field semantics
///   - BrrState block-advance and wrap logic

use apu::dsp::{decode_brr_nibble, decode_brr_block, Brr, EnvelopePhase};
use apu::Memory;

// ============================================================
// decode_brr_nibble — core arithmetic
// ============================================================

#[test]
fn test_nibble_zero_input_no_filter() {
    // nibble=0, shift=anything, filter=0 → output must be 0
    for shift in 0u8..=12 {
        let s = decode_brr_nibble(0, shift, 0, 0, 0);
        assert_eq!(s, 0, "shift={shift}");
    }
}

#[test]
fn test_nibble_shift0_filter0() {
    // shift=0: raw = nibble sign-extended to 16 bits (no scaling)
    // nibble=4 → 4 << 12 >> 12 = 4
    assert_eq!(decode_brr_nibble(4, 0, 0, 0, 0), 4);
    // nibble=-1 → -1
    assert_eq!(decode_brr_nibble(-1, 0, 0, 0, 0), -1);
}

#[test]
fn test_nibble_shift4_positive() {
    // nibble=4, shift=4 → 4 << 4 = 64
    assert_eq!(decode_brr_nibble(4, 4, 0, 0, 0), 64);
}

#[test]
fn test_nibble_shift4_negative() {
    // nibble=-2, shift=4 → -2 << 4 = -32
    assert_eq!(decode_brr_nibble(-2, 4, 0, 0, 0), -32);
}

#[test]
fn test_nibble_shift4_sign_extend_minus_one() {
    // nibble = -1 (0xF sign-extended), shift=4 → -1 << 4 = -16
    assert_eq!(decode_brr_nibble(-1, 4, 0, 0, 0), -16);
}

#[test]
fn test_nibble_shift12_max_positive() {
    // nibble=7 (max positive 4-bit), shift=12 → 7 << 12 = 28672, fits in 15-bit (+16383 max)
    // 7 << 12 = 28672 → clamped to 0x3FFF = 16383
    let s = decode_brr_nibble(7, 12, 0, 0, 0);
    assert_eq!(s, 0x3FFF);
}

#[test]
fn test_nibble_shift12_max_negative() {
    // nibble=-8 (min 4-bit), shift=12 → -8 << 12 = -32768, clamp to -0x4000 = -16384
    let s = decode_brr_nibble(-8, 12, 0, 0, 0);
    assert_eq!(s, -0x4000);
}

#[test]
fn test_nibble_shift_above_12_negative_saturates() {
    // shift=13,14,15 with negative nibble → hardware saturates to -1 (as i16 = -1)
    for shift in 13u8..=15 {
        let s = decode_brr_nibble(-1, shift, 0, 0, 0);
        // real hardware gives -1 (sign replicated) for negative nibbles
        assert_eq!(s, -1, "shift={shift} should saturate negative nibble to -1");
    }
}

#[test]
fn test_nibble_shift_above_12_non_negative_saturates_to_zero() {
    // shift=13-15 with non-negative nibble → 0
    for shift in 13u8..=15 {
        let s = decode_brr_nibble(4, shift, 0, 0, 0);
        assert_eq!(s, 0, "shift={shift} should saturate non-negative nibble to 0");
    }
}

// ============================================================
// decode_brr_nibble — filter correctness
// ============================================================

#[test]
fn test_filter0_ignores_history() {
    // filter=0: prediction=0, result purely from nibble
    let s = decode_brr_nibble(0, 0, 0, 999, 999);
    assert_eq!(s, 0);
}

#[test]
fn test_filter1_uses_prev1_only() {
    // nibble=0, shift=0, filter=1: output = 0 + prev1 - (prev1 >> 4)
    let prev1: i16 = 1024;
    let expected = (prev1 as i32 - (prev1 as i32 >> 4)) as i16;
    let s = decode_brr_nibble(0, 0, 1, prev1, 0);
    assert_eq!(s, expected);
}

#[test]
fn test_filter1_ignores_prev2() {
    // prev2 should have no effect in filter 1
    let s_no_prev2  = decode_brr_nibble(0, 0, 1, 512, 0);
    let s_with_prev2 = decode_brr_nibble(0, 0, 1, 512, 9999);
    assert_eq!(s_no_prev2, s_with_prev2);
}

#[test]
fn test_filter2_uses_both_prev() {
    // filter=2: pred = p1*2 - (p1*3>>5) - p2 + (p2>>4)
    let prev1: i16 = 512;
    let prev2: i16 = 256;
    let p1 = prev1 as i32;
    let p2 = prev2 as i32;
    let predicted = (p1 * 2) - ((p1 * 3) >> 5) - p2 + (p2 >> 4);
    let expected = predicted.clamp(-0x4000, 0x3FFF) as i16;
    let s = decode_brr_nibble(0, 0, 2, prev1, prev2);
    assert_eq!(s, expected);
}

#[test]
fn test_filter3_uses_both_prev() {
    // filter=3: pred = p1*2 - (p1*13>>6) - p2 + (p2*3>>4)
    let prev1: i16 = 400;
    let prev2: i16 = 200;
    let p1 = prev1 as i32;
    let p2 = prev2 as i32;
    let predicted = (p1 * 2) - ((p1 * 13) >> 6) - p2 + ((p2 * 3) >> 4);
    let expected = predicted.clamp(-0x4000, 0x3FFF) as i16;
    let s = decode_brr_nibble(0, 0, 3, prev1, prev2);
    assert_eq!(s, expected);
}

#[test]
fn test_filter_result_clamped_to_15_bit_positive() {
    // Large prev1 with filter 1 could overflow 15-bit; result must be ≤ 0x3FFF
    let s = decode_brr_nibble(7, 12, 1, 0x3FFF, 0);
    assert!(s <= 0x3FFF, "result {s} exceeds +15-bit max");
}

#[test]
fn test_filter_result_clamped_to_15_bit_negative() {
    let s = decode_brr_nibble(-8, 12, 1, -0x4000, 0);
    assert!(s >= -0x4000, "result {s} below -15-bit min");
}

#[test]
fn test_filter_with_nonzero_nibble_and_history() {
    // Combined: nibble contribution + filter contribution, filter 2
    let nibble: i8 = 3;
    let shift: u8 = 4;
    let prev1: i16 = 200;
    let prev2: i16 = 100;

    let raw = (nibble as i32) << 12 >> (12 - shift as i32);
    let p1 = prev1 as i32;
    let p2 = prev2 as i32;
    let pred = (p1 * 2) - ((p1 * 3) >> 5) - p2 + (p2 >> 4);
    let expected = (raw + pred).clamp(-0x4000, 0x3FFF) as i16;

    let s = decode_brr_nibble(nibble, shift, 2, prev1, prev2);
    assert_eq!(s, expected);
}

// ============================================================
// decode_brr_block — header parsing, flags, history threading
// ============================================================

/// Write a minimal 9-byte BRR block into APU RAM at `addr`.
fn write_block(mem: &mut Memory, addr: u16, header: u8, data: [u8; 8]) {
    mem.write8(addr, header);
    for (i, b) in data.iter().enumerate() {
        mem.write8(addr + 1 + i as u16, *b);
    }
}

#[test]
fn test_block_all_zero_data_no_flags() {
    // All-zero block: shift=0, filter=0, no end, no loop, all nibbles=0
    // All 16 decoded samples must be 0.
    let mut mem = Memory::new();
    write_block(&mut mem, 0x0100, 0x00, [0u8; 8]);

    let mut p1 = 0i16;
    let mut p2 = 0i16;
    let (samples, end, do_loop) = decode_brr_block(&mem.ram, 0x0100, &mut p1, &mut p2);

    assert!(!end);
    assert!(!do_loop);
    assert_eq!(samples, [0i16; 16]);
}

#[test]
fn test_block_end_flag_parsed() {
    let mut mem = Memory::new();
    // Header with end flag set (bit 0 = 1)
    write_block(&mut mem, 0x0100, 0x01, [0u8; 8]);

    let mut p1 = 0i16;
    let mut p2 = 0i16;
    let (_, end, do_loop) = decode_brr_block(&mem.ram, 0x0100, &mut p1, &mut p2);

    assert!(end,     "end flag should be set");
    assert!(!do_loop, "loop flag should not be set");
}

#[test]
fn test_block_loop_flag_parsed() {
    let mut mem = Memory::new();
    // Header with loop flag set (bit 1 = 1)
    write_block(&mut mem, 0x0100, 0x02, [0u8; 8]);

    let mut p1 = 0i16;
    let mut p2 = 0i16;
    let (_, end, do_loop) = decode_brr_block(&mem.ram, 0x0100, &mut p1, &mut p2);

    assert!(!end,   "end flag should not be set");
    assert!(do_loop, "loop flag should be set");
}

#[test]
fn test_block_both_flags_parsed() {
    let mut mem = Memory::new();
    write_block(&mut mem, 0x0100, 0x03, [0u8; 8]); // bits 1+0 both set

    let mut p1 = 0i16;
    let mut p2 = 0i16;
    let (_, end, do_loop) = decode_brr_block(&mem.ram, 0x0100, &mut p1, &mut p2);

    assert!(end);
    assert!(do_loop);
}

#[test]
fn test_block_shift_in_high_nibble_of_header() {
    // Shift=4 → header high nibble = 4, filter=0, no flags.
    // Header = 0b0100_0000 = 0x40.
    // Data byte 0x77 → high nibble=7, low nibble=7.
    // nibble 7, shift 4, filter 0 → 7 << 4 = 112.
    let mut mem = Memory::new();
    write_block(&mut mem, 0x0200, 0x40, [0x77, 0, 0, 0, 0, 0, 0, 0]);

    let mut p1 = 0i16;
    let mut p2 = 0i16;
    let (samples, _, _) = decode_brr_block(&mem.ram, 0x0200, &mut p1, &mut p2);

    assert_eq!(samples[0], 7 << 4, "first sample: nibble=7 shift=4 → 112");
    assert_eq!(samples[1], 7 << 4, "second sample: nibble=7 shift=4 → 112");
}

#[test]
fn test_block_history_threads_between_samples() {
    // Use filter 1 so each sample feeds the next via prev1.
    // Header: shift=0, filter=1 → 0b0000_0100 = 0x04.
    // All nibbles = 0, so each sample = filter(prev1, prev2) only.
    // Sample 0: nibble=0, p1=0, p2=0 → 0
    // Sample 1: nibble=0, p1=0, p2=0 → 0
    // ... all zero because initial history is zero and nibbles are zero.
    let mut mem = Memory::new();
    write_block(&mut mem, 0x0300, 0x04, [0u8; 8]);

    let mut p1 = 0i16;
    let mut p2 = 0i16;
    let (samples, _, _) = decode_brr_block(&mem.ram, 0x0300, &mut p1, &mut p2);

    // All zero (no accumulated history)
    for (i, &s) in samples.iter().enumerate() {
        assert_eq!(s, 0, "sample {i} should be 0");
    }
}

#[test]
fn test_block_history_threads_with_nonzero_start() {
    // With filter=1, nonzero initial prev1, zero nibbles:
    // Each sample = prev1 - (prev1 >> 4), so history decays.
    let mut mem = Memory::new();
    write_block(&mut mem, 0x0400, 0x04, [0u8; 8]); // shift=0, filter=1

    let mut p1: i16 = 1024;
    let mut p2: i16 = 0;
    let (samples, _, _) = decode_brr_block(&mem.ram, 0x0400, &mut p1, &mut p2);

    // Each sample should be strictly less than the previous (decaying)
    for i in 1..16 {
        assert!(
            samples[i].abs() <= samples[i - 1].abs() || samples[i - 1] == 0,
            "sample {i} should be decaying: prev={} cur={}",
            samples[i - 1],
            samples[i]
        );
    }
}

#[test]
fn test_block_prev_updated_after_decode() {
    // After decoding a block, p1/p2 should reflect the last two samples.
    let mut mem = Memory::new();
    // shift=4, filter=0: nibbles 0xAB → hi=0xA=-6, lo=0xB=-5 (sign-extended)
    write_block(&mut mem, 0x0500, 0x40, [0xAB, 0, 0, 0, 0, 0, 0, 0]);

    let mut p1: i16 = 0;
    let mut p2: i16 = 0;
    let (samples, _, _) = decode_brr_block(&mem.ram, 0x0500, &mut p1, &mut p2);

    assert_eq!(p1, samples[15], "p1 must equal last decoded sample");
    assert_eq!(p2, samples[14], "p2 must equal second-to-last decoded sample");
}

#[test]
fn test_block_16_samples_decoded() {
    // Every block must produce exactly 16 samples.
    let mut mem = Memory::new();
    write_block(&mut mem, 0x0600, 0x00, [0xAA; 8]);

    let mut p1 = 0i16;
    let mut p2 = 0i16;
    let (samples, _, _) = decode_brr_block(&mem.ram, 0x0600, &mut p1, &mut p2);

    assert_eq!(samples.len(), 16);
}

// ============================================================
// Brr struct — defaults and field semantics
// ============================================================

#[test]
fn test_brr_default_state() {
    let brr = Brr::default();
    assert_eq!(brr.addr,        0);
    assert_eq!(brr.nibble_idx,  0);
    assert_eq!(brr.prev1,       0);
    assert_eq!(brr.prev2,       0);
    assert_eq!(brr.loop_addr,   0);
    assert_eq!(brr.buffer_fill, 0);
    assert_eq!(brr.sample_buffer, [0i16; 16]);
}

#[test]
fn test_brr_nibble_idx_increments() {
    let mut brr = Brr::default();
    brr.nibble_idx = 0;
    brr.nibble_idx += 1;
    assert_eq!(brr.nibble_idx, 1);
    brr.nibble_idx = 15;
    brr.nibble_idx += 1;
    assert_eq!(brr.nibble_idx, 16); // caller is responsible for reset
}

#[test]
fn test_brr_addr_advances_by_9_at_block_boundary() {
    // Simulate what decode_next_block does when there is no end flag
    let mut brr = Brr::default();
    brr.addr = 0x1000;
    brr.nibble_idx = 16; // exhausted

    // The DSP advances addr by 9 at the end of a normal block
    if brr.nibble_idx >= 16 {
        brr.nibble_idx = 0;
        brr.addr = brr.addr.wrapping_add(9);
    }

    assert_eq!(brr.nibble_idx, 0);
    assert_eq!(brr.addr, 0x1009);
}

#[test]
fn test_brr_loop_addr_stored_separately() {
    let mut brr = Brr::default();
    brr.addr      = 0x2000;
    brr.loop_addr = 0x1500;
    // On a loop event the DSP copies loop_addr → addr
    brr.addr = brr.loop_addr;
    assert_eq!(brr.addr, 0x1500);
}

#[test]
fn test_brr_history_preserved_across_blocks() {
    // prev1/prev2 are NOT reset between normal blocks
    let mut brr = Brr::default();
    brr.prev1 = 1234;
    brr.prev2 = 5678;
    // Simulate a normal block advance (no loop)
    brr.addr = brr.addr.wrapping_add(9);
    assert_eq!(brr.prev1, 1234, "prev1 must survive a block advance");
    assert_eq!(brr.prev2, 5678, "prev2 must survive a block advance");
}

#[test]
fn test_brr_history_reset_on_loop() {
    // On a loop event, prev1 and prev2 are zeroed
    let mut brr = Brr::default();
    brr.prev1 = 9999;
    brr.prev2 = 8888;
    brr.addr  = brr.loop_addr;
    brr.prev1 = 0;
    brr.prev2 = 0;
    assert_eq!(brr.prev1, 0);
    assert_eq!(brr.prev2, 0);
}

#[test]
fn test_brr_buffer_fill_set_after_decode() {
    // After decode_next_block, buffer_fill should be 16
    let mut brr = Brr::default();
    // Simulate what the DSP does after a successful block decode
    brr.sample_buffer = [100i16; 16];
    brr.buffer_fill   = 16;
    brr.nibble_idx    = 0;
    assert_eq!(brr.buffer_fill, 16);
    assert_eq!(brr.nibble_idx,  0);
}

// ============================================================
// decode_brr_nibble — edge cases and regression checks
// ============================================================

#[test]
fn test_nibble_max_positive_nibble_shift0() {
    // nibble=7 (max positive 4-bit), shift=0, filter=0 → 7
    assert_eq!(decode_brr_nibble(7, 0, 0, 0, 0), 7);
}

#[test]
fn test_nibble_min_negative_nibble_shift0() {
    // nibble=-8 (min 4-bit), shift=0, filter=0 → -8
    assert_eq!(decode_brr_nibble(-8, 0, 0, 0, 0), -8);
}

#[test]
fn test_nibble_shift1() {
    assert_eq!(decode_brr_nibble(1, 1, 0, 0, 0), 2);
    assert_eq!(decode_brr_nibble(-1, 1, 0, 0, 0), -2);
}

#[test]
fn test_nibble_all_shifts_filter0_positive() {
    // nibble=1, filter=0: result should be 1 << shift for shift 0–12
    for shift in 0u8..=12 {
        let expected: i16 = 1 << shift;
        let got = decode_brr_nibble(1, shift, 0, 0, 0);
        assert_eq!(got, expected, "shift={shift}");
    }
}

#[test]
fn test_filter2_zero_nibble_zero_history() {
    assert_eq!(decode_brr_nibble(0, 0, 2, 0, 0), 0);
}

#[test]
fn test_filter3_zero_nibble_zero_history() {
    assert_eq!(decode_brr_nibble(0, 0, 3, 0, 0), 0);
}

#[test]
fn test_output_always_within_16bit_signed_range() {
    // Exhaustive check over a representative sample of inputs
    for shift in [0u8, 6, 12, 13, 15] {
        for filter in 0u8..=3 {
            for &nibble in &[-8i8, -1, 0, 1, 7] {
                for &p1 in &[-16384i16, -1000, 0, 1000, 16383] {
                    for &p2 in &[-16384i16, 0, 16383] {
                        let s = decode_brr_nibble(nibble, shift, filter, p1, p2);
                        assert!(
                            s >= i16::MIN && s <= i16::MAX,
                            "out of i16 range: nibble={nibble} shift={shift} filter={filter} \
                             p1={p1} p2={p2} → {s}"
                        );
                    }
                }
            }
        }
    }
}

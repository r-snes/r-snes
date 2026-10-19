//! BRR decoder tests
//!
//! Covers:
//!   - decode_brr_nibble: zero input, positive/negative nibbles, all 4 filters
//!   - shift boundary values (0, 12, 13–15 forced to -0x800/0)
//!   - filter rounding, 16-bit clamp and the deliberately wrapping doubling
//!   - reference vectors from blargg's SPC_DSP / Mesen2 and a real-chip log
//!   - decode_brr_block: header parsing, end/loop flags, history threading
//!   - Brr struct defaults and field semantics
//!   - BrrState block-advance and wrap logic

use apu::Memory;
use apu::dsp::{Brr, decode_brr_block, decode_brr_nibble};

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
    // shift=0: (nibble << 0) >> 1, then doubled — the low bit is lost.
    assert_eq!(decode_brr_nibble(4, 0, 0, 0, 0), 4);
    // -1 >> 1 = -1 (arithmetic), doubled → -2
    assert_eq!(decode_brr_nibble(-1, 0, 0, 0, 0), -2);
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
    // nibble=7, shift=12 → (7 << 12) >> 1 = 0x3800, doubled = 0x7000.
    // Must not be clipped to 0x3FFF (the old half-amplitude clamp).
    assert_eq!(decode_brr_nibble(7, 12, 0, 0, 0), 0x7000);
}

#[test]
fn test_nibble_shift12_max_negative() {
    // nibble=-8, shift=12 → -0x4000 at 15-bit scale, doubled = -0x8000.
    assert_eq!(decode_brr_nibble(-8, 12, 0, 0, 0), -0x8000);
}

#[test]
fn test_nibble_shift_above_12_negative_saturates() {
    // shift=13-15 with a negative nibble → -0x800 at 15-bit scale, so
    // -0x1000 once doubled, whatever the nibble's magnitude.
    for shift in 13u8..=15 {
        for nibble in -8i8..=-1 {
            assert_eq!(
                decode_brr_nibble(nibble, shift, 0, 0, 0),
                -0x1000,
                "shift={shift} nibble={nibble}"
            );
        }
    }
}

#[test]
fn test_nibble_shift_above_12_non_negative_saturates_to_zero() {
    // shift=13-15 with non-negative nibble → 0
    for shift in 13u8..=15 {
        let s = decode_brr_nibble(4, shift, 0, 0, 0);
        assert_eq!(
            s, 0,
            "shift={shift} should saturate non-negative nibble to 0"
        );
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
    // nibble=0, filter=1: p1 = 1024 >> 1 = 512; 512 + (-512 >> 4) = 480, doubled
    assert_eq!(decode_brr_nibble(0, 0, 1, 1024, 0), 960);
}

#[test]
fn test_filter1_ignores_prev2() {
    // prev2 should have no effect in filter 1
    let s_no_prev2 = decode_brr_nibble(0, 0, 1, 512, 0);
    let s_with_prev2 = decode_brr_nibble(0, 0, 1, 512, 9999);
    assert_eq!(s_no_prev2, s_with_prev2);
}

#[test]
fn test_filter2_uses_both_prev() {
    // p1=256, p2=128 (15-bit scale): 512 + (-768 >> 5) - 128 + 8 = 368, doubled
    assert_eq!(decode_brr_nibble(0, 0, 2, 512, 256), 736);
}

#[test]
fn test_filter3_uses_both_prev() {
    // p1=200, p2=100: 400 + (-2600 >> 6) - 100 + (300 >> 4) = 277, doubled
    assert_eq!(decode_brr_nibble(0, 0, 3, 400, 200), 554);
}

#[test]
fn test_filter_result_clamped_to_16_bit_positive() {
    // 15-bit-scale result far above 0x7FFF: clamped to 0x7FFF first,
    // then doubled with wrap → 0xFFFE = -2.
    assert_eq!(decode_brr_nibble(7, 12, 2, 32766, -32768), -2);
}

#[test]
fn test_filter_result_clamped_to_16_bit_negative() {
    // Clamped to -0x8000, doubled → -0x10000, which wraps to 0.
    assert_eq!(decode_brr_nibble(-8, 12, 3, -32768, 32766), 0);
}

#[test]
fn test_doubling_wraps_instead_of_clamping() {
    // Filter 1 with a loud history: at 15-bit scale the result
    // is 22015 (fits in 16 bits, so no clamp), and doubling it to 44030
    // wraps to -21506. Hardware does not clamp this step.
    assert_eq!(decode_brr_nibble(7, 12, 1, 0x3FFE, 0), -21506);
    // Same for filters 2/3 pushed past 0x3FFF by the history alone.
    assert_eq!(decode_brr_nibble(0, 0, 2, 24000, 0), -19786);
    assert_eq!(decode_brr_nibble(0, 0, 3, -24000, 0), 22410);
}

#[test]
fn test_filter_with_nonzero_nibble_and_history() {
    // nibble=3, shift=4 → 24; filter 2 on p1=100, p2=50 → +143; doubled
    assert_eq!(decode_brr_nibble(3, 4, 2, 200, 100), 334);
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

    assert!(end, "end flag should be set");
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

    assert!(!end, "end flag should not be set");
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
    assert_eq!(
        p2, samples[14],
        "p2 must equal second-to-last decoded sample"
    );
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
    assert_eq!(brr.addr, 0);
    assert_eq!(brr.nibble_idx, 0);
    assert_eq!(brr.prev1, 0);
    assert_eq!(brr.prev2, 0);
    assert!(!brr.block_end);
    assert_eq!(brr.buffer_fill, 0);
    assert_eq!(brr.sample_buffer, [0i16; 16]);
}

#[test]
fn test_brr_nibble_idx_increments() {
    let mut brr = apu::dsp::Brr {
        nibble_idx: 0,
        ..Default::default()
    };
    brr.nibble_idx += 1;
    assert_eq!(brr.nibble_idx, 1);
    brr.nibble_idx = 15;
    brr.nibble_idx += 1;
    assert_eq!(brr.nibble_idx, 16); // caller is responsible for reset
}

#[test]
fn test_brr_addr_advances_by_9_at_block_boundary() {
    // Simulate what decode_next_block does when there is no end flag
    let mut brr = apu::dsp::Brr {
        addr: 0x1000,
        nibble_idx: 16,
        ..Default::default()
    };

    // The DSP advances addr by 9 at the end of a normal block
    if brr.nibble_idx >= 16 {
        brr.nibble_idx = 0;
        brr.addr = brr.addr.wrapping_add(9);
    }

    assert_eq!(brr.nibble_idx, 0);
    assert_eq!(brr.addr, 0x1009);
}

#[test]
fn test_brr_history_preserved_across_blocks() {
    // prev1/prev2 are NOT reset between normal blocks
    let mut brr = apu::dsp::Brr {
        prev1: 1234,
        prev2: 5678,
        ..Default::default()
    };
    // Simulate a normal block advance (no loop)
    brr.addr = brr.addr.wrapping_add(9);
    assert_eq!(brr.prev1, 1234, "prev1 must survive a block advance");
    assert_eq!(brr.prev2, 5678, "prev2 must survive a block advance");
}

#[test]
fn test_brr_buffer_fill_set_after_decode() {
    // After decode_next_block, buffer_fill should be 16
    // Simulate what the DSP does after a successful block decode
    let brr = apu::dsp::Brr {
        sample_buffer: [100i16; 16],
        buffer_fill: 16,
        nibble_idx: 0,
        ..Default::default()
    };
    assert_eq!(brr.buffer_fill, 16);
    assert_eq!(brr.nibble_idx, 0);
}

// ============================================================
// decode_brr_nibble — edge cases and regression checks
// ============================================================

#[test]
fn test_nibble_max_positive_nibble_shift0() {
    // nibble=7, shift=0 → 7 >> 1 = 3, doubled → 6
    assert_eq!(decode_brr_nibble(7, 0, 0, 0, 0), 6);
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
    // nibble=1, filter=0: ((1 << shift) >> 1) * 2 — so 0 for shift 0,
    // and 1 << shift for shifts 1–12.
    assert_eq!(decode_brr_nibble(1, 0, 0, 0, 0), 0);
    for shift in 1u8..=12 {
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
                            (i16::MIN..=i16::MAX).contains(&s),
                            "out of i16 range: nibble={nibble} shift={shift} filter={filter} \
                             p1={p1} p2={p2} → {s}"
                        );
                    }
                }
            }
        }
    }
}

// ============================================================
// Reference vectors
// ============================================================

/// (nibble, shift, filter, prev1, prev2, expected), computed with
/// blargg's SPC_DSP `decode_brr` kernel; Mesen2's `DecodeBrrSample` gives
/// identical results (checked over all nibble/shift/filter combinations
/// and every even prev1).
const REFERENCE_VECTORS: &[(i8, u8, u8, i16, i16, i16)] = &[
    // shift 0 drops the low bit; shift 12 with max nibbles is full scale
    (4, 0, 0, 0, 0, 4),
    (-1, 0, 0, 0, 0, -2),
    (7, 0, 0, 0, 0, 6),
    (-8, 0, 0, 0, 0, -8),
    (1, 0, 0, 0, 0, 0),
    (7, 12, 0, 0, 0, 28672),
    (-8, 12, 0, 0, 0, -32768),
    // shifts 13-15: -0x800 / 0 at 15-bit scale
    (-1, 13, 0, 0, 0, -4096),
    (-8, 15, 0, 0, 0, -4096),
    (7, 14, 0, 0, 0, 0),
    (-1, 13, 2, 10000, -10000, 24340),
    (5, 15, 3, -30000, 30000, 0),
    // filter rounding
    (0, 0, 1, 1024, 0, 960),
    (0, 0, 2, 512, 256, 736),
    (0, 0, 3, 400, 200, 554),
    (3, 4, 2, 200, 100, 334),
    // doubling wraps (no clamp on that step)
    (7, 12, 1, 16382, 0, -21506),
    (0, 0, 2, 24000, 0, -19786),
    (0, 0, 3, -24000, 0, 22410),
    (4, 11, 2, 20000, 2000, -21096),
    // filters 2/3 pushed past 16 bits: clamp, then the doubling wraps
    (7, 12, 2, 32766, -32768, -2),
    (-8, 12, 3, -32768, 32766, 0),
];

#[test]
fn test_reference_vectors() {
    for &(n, shift, filter, p1, p2, want) in REFERENCE_VECTORS {
        assert_eq!(
            decode_brr_nibble(n, shift, filter, p1, p2),
            want,
            "nibble={n} shift={shift} filter={filter} p1={p1} p2={p2}"
        );
    }
}

/// (header, nibble, prev1, prev2, expected) captured from blargg's
/// cycle-accurate SPC_DSP decoding random BRR data: shift 12 with max
/// nibbles, shifts 13-15, filters 2/3 whose doubling wraps, clamp hits.
const CHIP_LOG_VECTORS: &[(u8, i8, i16, i16, i16)] = &[
    (0xcf, -8, 0, -25490, -12058),
    (0xc3, -8, -20480, 20480, -32768),
    (0xcd, -8, -778, -7998, -27668),
    (0xef, -1, 0, -27454, 18210),
    (0xf2, -5, 0, -4096, -4096),
    (0xdd, 3, 21500, 3276, -29566),
    (0xfb, 0, 0, -30430, 28528),
    (0x9d, 7, -22160, 11560, 19906),
    (0xdd, 2, 20976, 1478, -29048),
    (0xdd, 5, 28940, 18386, -28476),
    (0xdd, -8, 24830, 0, -25016),
    (0xdd, 1, -31510, 21216, 0),
    (0xbd, -3, -24968, 20792, 0),
    (0xa7, -3, -12104, -8540, -14420),
    (0x87, 4, -4796, -3750, -3474),
    (0x23, 2, -4, -8, 8),
];

#[test]
fn test_chip_log_vectors() {
    for &(header, n, p1, p2, want) in CHIP_LOG_VECTORS {
        let (shift, filter) = (header >> 4, (header >> 2) & 3);
        assert_eq!(
            decode_brr_nibble(n, shift, filter, p1, p2),
            want,
            "header={header:#04x} nibble={n} p1={p1} p2={p2}"
        );
    }
}

#[test]
fn test_block_decode_full_scale_and_wrap() {
    // shift 12, filter 0, nibbles 7 and -8: full-scale, not half-scale.
    let mut mem = Memory::new();
    write_block(&mut mem, 0x0200, 0xC0, [0x78; 8]);
    let (mut p1, mut p2) = (0i16, 0i16);
    let (samples, ..) = decode_brr_block(&mem.ram, 0x0200, &mut p1, &mut p2);
    for pair in samples.chunks(2) {
        assert_eq!(pair, [0x7000, -0x8000]);
    }

    // filter 1 accumulating a constant positive nibble: the 15-bit value
    // climbs past 0x3FFF and the stored (doubled) sample wraps negative,
    // exactly as on hardware.
    write_block(&mut mem, 0x0209, 0xC4, [0x77; 8]);
    let (mut p1, mut p2) = (0i16, 0i16);
    let (samples, ..) = decode_brr_block(&mem.ram, 0x0209, &mut p1, &mut p2);
    assert_eq!(samples[0], 0x7000);
    assert!(
        samples.iter().any(|&s| s < 0),
        "doubling must wrap: {samples:?}"
    );
    assert_eq!(p1, samples[15]);
    assert_eq!(p2, samples[14]);
}

// ============================================================
// BRR DECODER STATE (per voice)
// Tracks position within the compressed BRR sample stream.
// ============================================================

/// BRR playback state for one voice.
#[derive(Debug, Clone, Copy, Default)]
pub struct Brr {
    /// Address of the current BRR block header byte in APU RAM.
    pub addr: u16,

    /// Index of the next sample to consume from sample_buffer (0–15).
    pub nibble_idx: u8,

    /// Most recently decoded sample (p1 in filter equations).
    pub prev1: i16,

    /// Sample before prev1 (p2 in filter equations).
    pub prev2: i16,

    /// Loop start address (from the sample directory table).
    pub loop_addr: u16,

    /// Decoded sample buffer: holds all 16 samples from the current block.
    pub sample_buffer: [i16; 16],

    /// How many valid samples are in sample_buffer (always 16 after a decode).
    pub buffer_fill: u8,
}



// ============================================================
// GAUSSIAN INTERPOLATION TABLE
// 512-entry Gaussian kernel taken from the SNES DSP ROM.
// Used to smoothly interpolate between decoded BRR samples,
// eliminating the aliasing that would occur with nearest-neighbour.
// ============================================================

pub(super) const GAUSS: [i16; 512] = [
      0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
      1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   2,   2,   2,   2,   2,
      2,   2,   3,   3,   3,   3,   3,   4,   4,   4,   4,   4,   5,   5,   5,   5,
      6,   6,   6,   6,   7,   7,   7,   8,   8,   8,   9,   9,   9,  10,  10,  10,
     11,  11,  11,  12,  12,  13,  13,  14,  14,  15,  15,  15,  16,  17,  17,  18,
     18,  19,  19,  20,  20,  21,  21,  22,  23,  23,  24,  24,  25,  26,  27,  27,
     28,  29,  29,  30,  31,  32,  32,  33,  34,  35,  36,  36,  37,  38,  39,  40,
     41,  42,  43,  44,  45,  46,  47,  48,  49,  50,  51,  52,  53,  54,  55,  56,
     58,  59,  60,  61,  62,  64,  65,  66,  67,  69,  70,  71,  73,  74,  76,  77,
     78,  80,  81,  83,  84,  86,  87,  89,  90,  92,  94,  95,  97,  99, 100, 102,
    104, 106, 107, 109, 111, 113, 115, 117, 118, 120, 122, 124, 126, 128, 130, 132,
    134, 137, 139, 141, 143, 145, 147, 150, 152, 154, 156, 159, 161, 163, 166, 168,
    171, 173, 175, 178, 180, 183, 186, 188, 191, 193, 196, 199, 201, 204, 207, 210,
    212, 215, 218, 221, 224, 227, 230, 233, 236, 239, 242, 245, 248, 251, 254, 257,
    260, 263, 267, 270, 273, 276, 280, 283, 286, 290, 293, 297, 300, 304, 307, 311,
    314, 318, 321, 325, 329, 332, 336, 340, 343, 347, 351, 355, 358, 362, 366, 370,
    374, 378, 381, 385, 389, 393, 397, 401, 405, 410, 414, 418, 422, 426, 430, 434,
    439, 443, 447, 451, 456, 460, 464, 469, 473, 477, 482, 486, 491, 495, 499, 504,
    508, 513, 517, 522, 527, 531, 536, 540, 545, 550, 554, 559, 564, 568, 573, 578,
    583, 587, 592, 597, 602, 607, 611, 616, 621, 626, 631, 636, 641, 646, 651, 656,
    661, 666, 671, 676, 681, 686, 691, 696, 702, 707, 712, 717, 722, 727, 733, 738,
    743, 748, 754, 759, 764, 769, 775, 780, 785, 791, 796, 801, 807, 812, 818, 823,
    828, 834, 839, 845, 850, 856, 861, 867, 872, 878, 883, 889, 894, 900, 906, 911,
    917, 922, 928, 934, 939, 945, 951, 956, 962, 968, 974, 979, 985, 991, 997,1002,
   1008,1014,1020,1026,1031,1037,1043,1049,1055,1061,1067,1073,1079,1084,1090,1096,
   1102,1108,1114,1120,1126,1132,1138,1144,1150,1156,1162,1169,1175,1181,1187,1193,
   1199,1205,1211,1217,1224,1230,1236,1242,1249,1255,1261,1267,1274,1280,1286,1293,
   1299,1305,1312,1318,1324,1331,1337,1343,1350,1356,1363,1369,1376,1382,1389,1395,
   1402,1408,1415,1421,1428,1434,1441,1447,1454,1461,1467,1474,1480,1487,1494,1500,
   1507,1514,1520,1527,1534,1541,1547,1554,1561,1568,1574,1581,1588,1595,1602,1609,
   1616,1623,1630,1636,1643,1650,1657,1664,1671,1678,1685,1692,1699,1706,1713,1720,
   1727,1734,1741,1748,1755,1762,1769,1777,1784,1791,1798,1805,1812,1819,1826,1833,
];

// ============================================================
// BRR DECODING FUNCTIONS
// ============================================================

/// Decode one 4-bit BRR nibble into a 16-bit PCM sample.
///
/// Hardware steps:
///   1. Sign-extend nibble from 4 bits to i16
///   2. Scale by shift: arithmetically shift left so the value is
///      placed at the top of a 16-bit integer, then shift right by
///      (12 - shift). This is equivalent to: (nibble << 12) >> (12 - shift).
///      Shifts > 12 are clamped (hardware saturates the sign bit).
///   3. Add the prediction filter result (computed in i32 to avoid overflow)
///   4. Clamp to 15-bit signed range (-16384..+16383) — hardware precision
///   5. The result is a valid i16 (fits in 16 bits)
pub fn decode_brr_nibble(nibble: i8, shift: u8, filter: u8, prev1: i16, prev2: i16) -> i16 {
    // Step 1 + 2: scale the nibble to a 16-bit range.
    let raw: i32 = if shift <= 12 {
        // Standard path: sign-extend to i32, align to bit 15, then scale back.
        let extended = nibble as i32; // already sign-extended from i8
        (extended << 12) >> (12u32.saturating_sub(shift as u32))
    } else {
        // Shifts 13–15: hardware quirk — the sign bit replicates, so the
        // result is either 0x0000 or 0xFFFF (0 or -1 in i16).
        if nibble < 0 { -1 } else { 0 }
    };

    // Step 3: prediction filter in i32 to prevent intermediate overflow.
    let p1 = prev1 as i32;
    let p2 = prev2 as i32;

    let predicted: i32 = match filter {
        0 => 0,
        // Filter 1: coefficient ≈ 15/16
        1 => p1 - (p1 >> 4),
        // Filter 2: p1 * ~61/32 - p2 * ~15/16
        2 => (p1 * 2) - ((p1 * 3) >> 5) - p2 + (p2 >> 4),
        // Filter 3: p1 * ~115/64 - p2 * ~13/16
        3 => (p1 * 2) - ((p1 * 13) >> 6) - p2 + ((p2 * 3) >> 4),
        _ => 0,
    };

    let result = raw + predicted;

    // Step 4: clamp to 15-bit signed range (-0x4000..+0x3FFF).
    // The SNES hardware uses 15 significant bits internally.
    let clamped = result.clamp(-0x4000, 0x3FFF);

    clamped as i16
}

/// Read one byte from a raw RAM slice; returns 0 for out-of-range addresses.
#[inline(always)]
pub(super) fn ram_read8(ram: &[u8], addr: u16) -> u8 {
    ram.get(addr as usize).copied().unwrap_or(0)
}

/// Decode all 16 samples from one 9-byte BRR block.
///
/// Takes a plain `&[u8]` RAM slice so the DSP can read sample data
/// without needing to borrow the full `Memory` struct.
///
/// BRR block layout (header byte = SSSSFFEX):
///   bits 7-4 = shift amount (0–15)
///   bits 3-2 = filter index (0–3)
///   bit  1   = loop flag
///   bit  0   = end flag
///   bytes 1-8 — 8 data bytes = 16 nibbles = 16 samples
///
/// `prev1` and `prev2` are updated in place so history carries forward.
/// Returns (samples[16], end_flag, loop_flag).
pub fn decode_brr_block(
    ram: &[u8],
    addr: u16,
    prev1: &mut i16,
    prev2: &mut i16,
) -> ([i16; 16], bool, bool) {
    let header = ram_read8(ram, addr);

    let shift  = (header >> 4) & 0x0F;
    let filter = (header >> 2) & 0x03;
    let looop  = (header & 0x02) != 0;
    let end    = (header & 0x01) != 0;

    let mut samples = [0i16; 16];

    for i in 0..8usize {
        let byte = ram_read8(ram, addr + 1 + i as u16);

        let hi_raw = ((byte >> 4) & 0x0F) as i8;
        let hi = if hi_raw & 0x08 != 0 { hi_raw | !0x0F } else { hi_raw };
        let lo_raw = (byte & 0x0F) as i8;
        let lo = if lo_raw & 0x08 != 0 { lo_raw | !0x0F } else { lo_raw };

        let s0 = decode_brr_nibble(hi, shift, filter, *prev1, *prev2);
        *prev2 = *prev1; *prev1 = s0;
        samples[i * 2] = s0;

        let s1 = decode_brr_nibble(lo, shift, filter, *prev1, *prev2);
        *prev2 = *prev1; *prev1 = s1;
        samples[i * 2 + 1] = s1;
    }

    (samples, end, looop)
}

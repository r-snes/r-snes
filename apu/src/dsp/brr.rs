use crate::memory::RawARAM;

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

    /// Whether the block currently in `sample_buffer` had its end flag set.
    pub block_end: bool,

    /// Decoded sample buffer: holds all 16 samples from the current block.
    pub sample_buffer: [i16; 16],

    /// How many valid samples are in sample_buffer (always 16 after a decode).
    pub buffer_fill: u8,
}

/// 512-entry Gaussian kernel from the SNES DSP ROM.
///
/// Used to smoothly interpolate between decoded BRR samples,
/// eliminating the aliasing that would occur with nearest-neighbour
/// sample selection at non-native pitches.
///
/// Indexed in four 256-entry regions by an 8-bit fractional pitch
/// position `index` (0-255): `GAUSS[255-index]`, `GAUSS[511-index]`,
/// `GAUSS[index+256]`, `GAUSS[index]` weight the four most recent
/// decoded samples, oldest to newest. See `Voice::interpolate`.
pub(super) const GAUSS: [i16; 512] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2,
    2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10,
    10, 11, 11, 11, 12, 12, 13, 13, 14, 14, 15, 15, 15, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21,
    22, 23, 23, 24, 24, 25, 26, 27, 27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 36, 36, 37, 38, 39,
    40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 58, 59, 60, 61, 62, 64, 65,
    66, 67, 69, 70, 71, 73, 74, 76, 77, 78, 80, 81, 83, 84, 86, 87, 89, 90, 92, 94, 95, 97, 99,
    100, 102, 104, 106, 107, 109, 111, 113, 115, 117, 118, 120, 122, 124, 126, 128, 130, 132, 134,
    137, 139, 141, 143, 145, 147, 150, 152, 154, 156, 159, 161, 163, 166, 168, 171, 173, 175, 178,
    180, 183, 186, 188, 191, 193, 196, 199, 201, 204, 207, 210, 212, 215, 218, 221, 224, 227, 230,
    233, 236, 239, 242, 245, 248, 251, 254, 257, 260, 263, 267, 270, 273, 276, 280, 283, 286, 290,
    293, 297, 300, 304, 307, 311, 314, 318, 321, 325, 329, 332, 336, 340, 343, 347, 351, 355, 358,
    362, 366, 370, 374, 378, 381, 385, 389, 393, 397, 401, 405, 410, 414, 418, 422, 426, 430, 434,
    439, 443, 447, 451, 456, 460, 464, 469, 473, 477, 482, 486, 491, 495, 499, 504, 508, 513, 517,
    522, 527, 531, 536, 540, 545, 550, 554, 559, 564, 568, 573, 578, 583, 587, 592, 597, 602, 607,
    611, 616, 621, 626, 631, 636, 641, 646, 651, 656, 661, 666, 671, 676, 681, 686, 691, 696, 702,
    707, 712, 717, 722, 727, 733, 738, 743, 748, 754, 759, 764, 769, 775, 780, 785, 791, 796, 801,
    807, 812, 818, 823, 828, 834, 839, 845, 850, 856, 861, 867, 872, 878, 883, 889, 894, 900, 906,
    911, 917, 922, 928, 934, 939, 945, 951, 956, 962, 968, 974, 979, 985, 991, 997, 1002, 1008,
    1014, 1020, 1026, 1031, 1037, 1043, 1049, 1055, 1061, 1067, 1073, 1079, 1084, 1090, 1096, 1102,
    1108, 1114, 1120, 1126, 1132, 1138, 1144, 1150, 1156, 1162, 1169, 1175, 1181, 1187, 1193, 1199,
    1205, 1211, 1217, 1224, 1230, 1236, 1242, 1249, 1255, 1261, 1267, 1274, 1280, 1286, 1293, 1299,
    1305, 1312, 1318, 1324, 1331, 1337, 1343, 1350, 1356, 1363, 1369, 1376, 1382, 1389, 1395, 1402,
    1408, 1415, 1421, 1428, 1434, 1441, 1447, 1454, 1461, 1467, 1474, 1480, 1487, 1494, 1500, 1507,
    1514, 1520, 1527, 1534, 1541, 1547, 1554, 1561, 1568, 1574, 1581, 1588, 1595, 1602, 1609, 1616,
    1623, 1630, 1636, 1643, 1650, 1657, 1664, 1671, 1678, 1685, 1692, 1699, 1706, 1713, 1720, 1727,
    1734, 1741, 1748, 1755, 1762, 1769, 1777, 1784, 1791, 1798, 1805, 1812, 1819, 1826, 1833,
];

/// The BRR prediction filter, at the decoder's internal 15-bit scale.
///
/// `p1` and `p2` are the two previous samples *at 15-bit scale*, i.e. the
/// stored 16-bit samples shifted right by one (`stored >> 1`). Returns the
/// amount to add to the scaled nibble.
#[inline]
pub fn brr_filter(filter: u8, p1: i32, p2: i32) -> i32 {
    match filter & 3 {
        0 => 0,
        // 15/16
        1 => p1 + ((-p1) >> 4),
        // 61/32 * p1 - 15/16 * p2
        2 => (p1 << 1) + ((-(p1 * 3)) >> 5) - p2 + (p2 >> 4),
        // 115/64 * p1 - 13/16 * p2
        _ => (p1 << 1) + ((-(p1 * 13)) >> 6) - p2 + ((p2 * 3) >> 4),
    }
}

/// Decode one 4-bit BRR nibble into a 16-bit PCM sample.
///
/// `prev1` / `prev2` are the two previously *decoded* samples, exactly as
/// this function returned them (16-bit stored scale).
///
/// Hardware steps:
///   1. Sign-extend the nibble from 4 bits.
///   2. Scale: `(nibble << shift) >> 1` — a 15-bit-scale value.
///      Shifts 13–15 are invalid on hardware and force the value to
///      -0x800 for negative nibbles, 0 otherwise.
///   3. Add the prediction filter, computed on `prev >> 1`.
///   4. Clamp to 16 bits (-0x8000..=0x7FFF).
///   5. Double and truncate to i16. This doubling is *not* clamped: a
///      value above 0x3FFF wraps to negative. Some samples rely on this
///      wrap, so it must be reproduced exactly.
///
/// The returned sample is therefore always even.
pub fn decode_brr_nibble(nibble: i8, shift: u8, filter: u8, prev1: i16, prev2: i16) -> i16 {
    // Step 1: force a 4-bit sign extension even if the caller passed a
    // wider value.
    let n = ((nibble as i32) << 28) >> 28;

    // Step 2: scale to 15-bit scale.
    let mut s: i32 = if shift <= 12 {
        (n << shift) >> 1
    } else if n < 0 {
        -0x800
    } else {
        0
    };

    // Step 3: prediction filter on the halved history.
    s += brr_filter(filter, (prev1 as i32) >> 1, (prev2 as i32) >> 1);

    // Step 4: clamp to 16 bits, then step 5: double with wrap-around.
    (s.clamp(i16::MIN as i32, i16::MAX as i32) as i16).wrapping_mul(2)
}

/// Read one byte from a raw RAM slice; returns 0 for out-of-range addresses.
#[inline(always)]
pub(super) fn ram_read8(ram: &RawARAM, addr: u16) -> u8 {
    ram.get(addr as usize).copied().unwrap_or(0)
}

/// Decode all 16 samples from one 9-byte BRR block.
///
/// Takes a plain `&RawARAM` RAM slice so the DSP can read sample data
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
    ram: &RawARAM,
    addr: u16,
    prev1: &mut i16,
    prev2: &mut i16,
) -> ([i16; 16], bool, bool) {
    let header = ram_read8(ram, addr);

    let shift = (header >> 4) & 0x0F;
    let filter = (header >> 2) & 0x03;
    let looop = (header & 0x02) != 0;
    let end = (header & 0x01) != 0;

    let mut samples = [0i16; 16];

    for i in 0..8usize {
        let byte = ram_read8(ram, addr + 1 + i as u16);

        let hi_raw = ((byte >> 4) & 0x0F) as i8;
        let hi = if hi_raw & 0x08 != 0 {
            hi_raw | !0x0F
        } else {
            hi_raw
        };
        let lo_raw = (byte & 0x0F) as i8;
        let lo = if lo_raw & 0x08 != 0 {
            lo_raw | !0x0F
        } else {
            lo_raw
        };

        let s0 = decode_brr_nibble(hi, shift, filter, *prev1, *prev2);
        *prev2 = *prev1;
        *prev1 = s0;
        samples[i * 2] = s0;

        let s1 = decode_brr_nibble(lo, shift, filter, *prev1, *prev2);
        *prev2 = *prev1;
        *prev1 = s1;
        samples[i * 2 + 1] = s1;
    }

    (samples, end, looop)
}

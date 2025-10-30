/// Converts a 15-bit BGR555 color to a 32-bit ARGB color
///
/// The input format is a 16-bit integer where:
/// - Bits 0-4   = Red (5 bits)
/// - Bits 5-9   = Green (5 bits)
/// - Bits 10-14 = Blue (5 bits)
/// - Bit 15     = Unused/ignored
///
/// This function expands each 5-bit channel to 8 bits by duplicating the
/// upper bits, ensuring proper color scaling. The resulting color is
/// returned in 0xAARRGGBB format, with alpha always set to 255 (fully opaque)
///
/// # Parameters
/// - `bgr`: A 16-bit value representing a BGR555 color
///
/// # Returns
/// A 32-bit ARGB color as `u32`
pub fn bgr555_to_argb(bgr: u16) -> u32 {
    let r = (bgr & 0x1F) as u32;
    let g = ((bgr >> 5) & 0x1F) as u32;
    let b = ((bgr >> 10) & 0x1F) as u32;

    // Expand 5-bit to 8-bit by duplicating upper bits
    let r8 = (r << 3) | (r >> 2);
    let g8 = (g << 3) | (g >> 2);
    let b8 = (b << 3) | (b >> 2);

    (0xFF << 24) | (r8 << 16) | (g8 << 8) | b8
}

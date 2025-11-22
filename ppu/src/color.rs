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

#[cfg(test)]
mod tests_color {
    use super::*;

    #[test] // Converting pure black should return opaque black
    fn test_black_color() {
        let result = bgr555_to_argb(0x0000);
        assert_eq!(result, 0xFF000000);
    }

    #[test] // Converting pure red should return opaque red (0xFF0000)
    fn test_pure_red() {
        let result = bgr555_to_argb(0x001F); // R = 31
        assert_eq!(result & 0xFFFFFF, 0xFF0000);
    }

    #[test] // Converting pure green should return opaque green (0x00FF00)
    fn test_pure_green() {
        let result = bgr555_to_argb(0x03E0); // G = 31
        assert_eq!(result & 0xFFFFFF, 0x00FF00);
    }

    #[test] // Converting pure blue should return opaque blue (0x0000FF)
    fn test_pure_blue() {
        let result = bgr555_to_argb(0x7C00); // B = 31
        assert_eq!(result & 0xFFFFFF, 0x0000FF);
    }

    #[test] // Alpha channel should always be fully opaque (0xFF)
    fn test_alpha_is_always_opaque() {
        let colors = [0x0000, 0x7FFF, 0x1234, 0x6B5A];
        for &c in &colors {
            let result = bgr555_to_argb(c);
            assert_eq!(result >> 24, 0xFF);
        }
    }

    #[test] // Mid-gray tone should produce a middle RGB value around 0x808080
    fn test_mid_gray() {
        let result = bgr555_to_argb(0x3DEF); // roughly half of max per channel
        let r = (result >> 16) & 0xFF;
        let g = (result >> 8) & 0xFF;
        let b = result & 0xFF;
        assert!(r >= 120 && r <= 136);
        assert!(g >= 120 && g <= 136);
        assert!(b >= 120 && b <= 136);
    }
}

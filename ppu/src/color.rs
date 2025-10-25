pub fn bgr555_to_argb(bgr: u16) -> u32 {
    let r = (bgr & 0x1F) as u32;
    let g = ((bgr >> 5) & 0x1F) as u32;
    let b = ((bgr >> 10) & 0x1F) as u32;

    let r8 = (r << 3) | (r >> 2);
    let g8 = (g << 3) | (g >> 2);
    let b8 = (b << 3) | (b >> 2);

    (0xFF << 24) | (r8 << 16) | (g8 << 8) | b8
}

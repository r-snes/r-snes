use crate::ppu::*;

#[test] // CGRAM should have exactly 256 entries
fn test_cgram_initialization() {
    let ppu = PPU::new();
    assert_eq!(ppu.cgram_len(), 256);
}

#[test] // Setting CGRAM address should reset the latch
fn test_set_cgram_addr_resets_latch() {
    let mut ppu = PPU::new();
    ppu.write_cgram_data(0x12); // set latch
    ppu.set_cgram_addr(5);
    assert_eq!(ppu.get_cgaddr(), 5);
    assert!(!ppu.is_latch_set());
}

#[test] // Two consecutive writes should store a 16-bit color and clear latch
fn test_write_cgram_data_two_writes() {
    let mut ppu = PPU::new();
    ppu.set_cgram_addr(0);

    // First write
    ppu.write_cgram_data(0x34);
    assert!(ppu.is_latch_set());
    assert_eq!(ppu.get_cgram_value(0), 0); // not written yet

    // Second write
    ppu.write_cgram_data(0x12);
    assert!(!ppu.is_latch_set());
    assert_eq!(ppu.get_cgram_value(0), 0x1234 & 0x7FFF);
}

#[test] // CGRAM address should auto-increment after second write
fn test_write_cgram_auto_increment() {
    let mut ppu = PPU::new();
    ppu.set_cgram_addr(0);

    ppu.write_cgram_data(0x56); // low byte
    ppu.write_cgram_data(0x78); // high byte
    assert_eq!(ppu.get_cgaddr(), 1); // auto-incremented
}

#[test] // Reading CGRAM should return ARGB converted from BGR555
fn test_read_cgram_returns_argb() {
    let mut ppu = PPU::new();
    ppu.set_cgram_addr(0);
    ppu.write_cgram_data(0x1F); // low byte
    ppu.write_cgram_data(0x3F); // high byte
    let argb = ppu.read_cgram(0);
    assert_eq!(argb, bgr555_to_argb(0x3F1F & 0x7FFF));
}

#[test] // Multiple consecutive writes should store correct colors in CGRAM
fn test_multiple_colors() {
    let mut ppu = PPU::new();
    for i in 0..5 {
        let i_u8 = i as u8;
        ppu.set_cgram_addr(i_u8);
        ppu.write_cgram_data(i_u8);                 // low byte
        ppu.write_cgram_data(i_u8.wrapping_add(1)); // high byte
    }

    for i in 0..5 {
        let color: u16 = (((i + 1) << 8) | i) as u16;
        assert_eq!(ppu.get_cgram_value(i as usize), color & 0x7FFF);
    }
}

#[test] // Should correctly handle writing at the last CGRAM index
fn test_write_and_read_last_cgram_entry() {
    let mut ppu = PPU::new();
    let last_index = 255;
    ppu.set_cgram_addr(last_index as u8);
    ppu.write_cgram_data(0xAA);
    ppu.write_cgram_data(0xBB);
    assert_eq!(ppu.get_cgram_value(last_index), ((0xBBu16 << 8) | 0xAA) & 0x7FFF);
}

#[test] // Latch should clear when setting a new CGRAM address
fn test_latch_clears_on_set_address() {
    let mut ppu = PPU::new();
    ppu.write_cgram_data(0x55); // first write
    assert!(ppu.is_latch_set());
    ppu.set_cgram_addr(10);
    assert!(!ppu.is_latch_set());
}

use ppu::ppu::PPU;
use ppu::utils::VRAM_SIZE;

#[test] // Should return the value that was written at the address
fn test_write_and_read_valid_addr() {
    let mut ppu = PPU::new();
    ppu.write_vram(0x1234, 0xAB);
    assert_eq!(ppu.read_vram(0x1234), 0xAB);
}

#[test] // Reading out of bounds should return 0
fn test_read_invalid_addr() {
    let ppu = PPU::new();
    assert_eq!(ppu.read_vram(999999), 0x00);
}

#[test] // Should not panic, just a stderr messages, then return 0
fn test_write_invalid_addr() {
    let mut ppu = PPU::new();
    ppu.write_vram(999999, 0xAB);
    assert_eq!(ppu.read_vram(999999), 0x00);
}

#[test] // Should return the same value that was written
fn test_ppu_write_and_read_valid_address() {
    let mut ppu = PPU::new();
    ppu.write_vram(0x1234, 0xAB);
    let value = ppu.read_vram(0x1234);
    assert_eq!(value, 0xAB);
}

#[test] // Should return the most recent value written
fn test_ppu_write_overrides_previous_value() {
    let mut ppu = PPU::new();
    ppu.write_vram(0x1234, 0xAB);
    ppu.write_vram(0x1234, 0xCD);
    let value = ppu.read_vram(0x1234);
    assert_eq!(value, 0xCD);
}

#[test] // Should return 0 if nothing was written at this address
fn test_ppu_read_uninitialized_memory_returns_zero() {
    let ppu = PPU::new();
    let value = ppu.read_vram(0x1234);
    assert_eq!(value, 0x00);
}

#[test] // Should not panic even if out of bounds
fn test_ppu_write_invalid_address_does_not_panic() {
    let mut ppu = PPU::new();
    ppu.write_vram(VRAM_SIZE + 1, 0xAB);
}

#[test] // Should return 0 when reading beyond VRAM limits
fn test_ppu_read_invalid_address_returns_zero() {
    let ppu = PPU::new();
    let value = ppu.read_vram(VRAM_SIZE + 10);
    assert_eq!(value, 0x00);
}

#[test] // Should correctly write and read at the last valid VRAM address
fn test_ppu_write_and_read_at_last_valid_address() {
    let mut ppu = PPU::new();
    let addr = VRAM_SIZE - 1;
    ppu.write_vram(addr, 0xAB);
    let value = ppu.read_vram(addr);
    assert_eq!(value, 0xAB);
}

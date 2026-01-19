mod bus;
mod ppu;
mod vram;
mod oam;

use bus::Bus;

fn main() {
    let mut bus = Bus::new();

    // ---------------- VRAM ----------------

    let initial_data: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33,
        0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB,
        0xCC, 0xDD, 0xEE, 0xFF,
    ];
    bus.ppu.vram.load_from_slice(&initial_data);

    // Set VRAM address ($2116/$2117)
    bus.cpu_write(0x2116, 0x34);
    bus.cpu_write(0x2117, 0x12);

    // Write word via VRAM data ports ($2118/$2119)
    bus.cpu_write(0x2118, 0xCD); // low
    bus.cpu_write(0x2119, 0xAB); // high

    // Read via VRAM read ports ($2139/$213A)
    bus.cpu_write(0x2116, 0x34);
    bus.cpu_write(0x2117, 0x12);

    let first_read = bus.cpu_read(0x2139);
    let lo = bus.cpu_read(0x2139);
    let hi = bus.cpu_read(0x213A);
    let word = (hi as u16) << 8 | lo as u16;

    println!("=== VRAM TEST ===");
    println!("First read = {:02X}", first_read);
    println!("Read word = {:04X}", word);

    // ---------------- OAM SPRITES ----------------

    println!("\n=== OAM TEST (direct helpers) ===");

    let sprite = [0x10, 0x20, 0x30, 0x40];
    bus.ppu.oam.write_sprite(0, sprite);

    let s = bus.ppu.oam.read_sprite(0);
    println!("Sprite[0] = {:02X} {:02X} {:02X} {:02X}", s[0], s[1], s[2], s[3]);

    // ---------------- OAM BUS ----------------

    println!("\n=== OAM TEST (via PPU bus) ===");

    // Set OAM address ($2102/$2103)
    bus.cpu_write(0x2102, 0x00); // low
    bus.cpu_write(0x2103, 0x00); // high

    // Write 4 bytes (sprite 0)
    bus.cpu_write(0x2104, 0xAA);
    bus.cpu_write(0x2104, 0xBB);
    bus.cpu_write(0x2104, 0xCC);
    bus.cpu_write(0x2104, 0xDD);

    let s = bus.ppu.oam.read_sprite(0);
    println!("Sprite[0] after port write = {:02X} {:02X} {:02X} {:02X}", s[0], s[1], s[2], s[3]);

    // Read back via OAM read port ($2138)
    bus.cpu_write(0x2102, 0x00);
    bus.cpu_write(0x2103, 0x00);

    let r0 = bus.cpu_read(0x2138);
    let r1 = bus.cpu_read(0x2138);
    let r2 = bus.cpu_read(0x2138);
    let r3 = bus.cpu_read(0x2138);

    println!("OAM read via port = {:02X} {:02X} {:02X} {:02X}", r0, r1, r2, r3);
}

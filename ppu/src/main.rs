mod bus;
mod ppu;
mod vram;

use bus::Bus;

fn main() {
    let mut bus = Bus::new();

    // Initialize VRAM from a slice
    // Load some known pattern for testing (first 16 bytes)
    let initial_data: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33,
        0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB,
        0xCC, 0xDD, 0xEE, 0xFF,
    ];
    bus.ppu.vram.load_from_slice(&initial_data);

    // Set the VRAM address to 0x1234 via registers
    bus.cpu_write(0x2116, 0x34); // low byte
    bus.cpu_write(0x2117, 0x12); // high byte

    // Write a 16-bit word to VRAM via the data port
    bus.cpu_write(0x2118, 0xCD); // low byte
    bus.cpu_write(0x2118, 0xAB); // high byte -> commit

    // Set address and auto-increment directly using helper functions
    bus.ppu.vram.set_addr(0x1234);
    bus.ppu.vram.set_auto_increment(1);

    // Read back the word using the helper method read_word_via_port
    let first_read = bus.ppu.vram.read_data_port_byte(); // dummy read due to buffer
    let word_via_port = bus.ppu.vram.read_word_via_port(); // reads low then high byte
    let vram_addr = bus.ppu.vram.get_addr(); // get current VRAM address

    // Display the results
    println!("First buffered read = {:02X}", first_read);
    println!("VRAM word read via port = {:04X}", word_via_port);
    println!("Current VRAM address = {:04X}", vram_addr);

    // Verify direct memory contents
    let lo_direct = bus.ppu.vram.mem_read16_at(0x1234) & 0xFF;
    let hi_direct = (bus.ppu.vram.mem_read16_at(0x1234) >> 8) & 0xFF;
    println!("Direct memory read low byte = {:02X}", lo_direct);
    println!("Direct memory read high byte = {:02X}", hi_direct);
}

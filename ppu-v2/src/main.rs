mod registers;
mod vram;
mod constants;

use vram::VRAM;

fn main() {
    let mut vram = VRAM::new();

    // -----------------------------
    // Test VMAIN ($2115)
    // -----------------------------
    println!("--- Testing VMAIN ---");
    vram.write_vmain(0b0000_0000); // increment after low byte, increment mode 0
    println!("VMAIN = {:08b}", vram.vmain);
    println!("  increment_amount() = {}", vram.increment_amount());
    println!("  increment_after_low() = {}", vram.increment_after_low());
    println!("  increment_after_high() = {}", vram.increment_after_high());

    // -----------------------------
    // Test VMADD ($2116/$2117)
    // -----------------------------
    println!("\n--- Testing VMADD ---");
    vram.write_vmadd_low(0x34);
    vram.write_vmadd_high(0x01);
    println!("VMADD = 0x{:04X}", vram.vmadd);
    println!("Byte address = 0x{:04X}", vram.byte_address());
    println!("VRAM latch = 0x{:04X}", vram.vram_latch);

    // -----------------------------
    // Test VMDATAL / VMDATAH ($2118/$2119)
    // -----------------------------
    println!("\n--- Testing VMDATA write ---");

    vram.write_vmain(0x80);
    
    // Write a word 0xABCD at current VMADD
    vram.write_vmdatal(0xCD);
    vram.write_vmdatah(0xAB);

    // Check memory content (low byte / high byte)
    let mut addr = vram.byte_address() - 2; // previous address of the word written
    println!("Memory[0x{:04X}] (low1)  = 0x{:02X}", addr, vram.memory[addr]);
    println!("Memory[0x{:04X}] (high1) = 0x{:02X}", addr + 1, vram.memory[addr + 1]);
    println!("Memory[0x{:04X}] (low2)  = 0x{:02X}", addr + 2, vram.memory[addr + 2]);
    println!("Memory[0x{:04X}] (high2) = 0x{:02X}", addr + 3, vram.memory[addr + 3]);

    // -----------------------------
    // Test auto-increment
    // -----------------------------
    println!("\n--- Testing auto-increment ---");
    let before = vram.vmadd;
    println!("VMADD before write = 0x{:04X}", before);

    vram.write_vmdatal(0x11);
    vram.write_vmdatah(0x22);

    let after = vram.vmadd;
    println!("VMADD after write  = 0x{:04X}", after);
    println!(
        "Memory at new VMADD address = 0x{:02X} 0x{:02X}",
        vram.memory[vram.byte_address()],
        vram.memory[vram.byte_address() + 1]
    );

    // -----------------------------
    // Test repeated writes (write-twice behavior)
    // -----------------------------
    println!("\n--- Testing repeated writes ---");
    let start_addr = vram.vmadd;
    vram.write_vmdatal(0xAA);
    vram.write_vmdatah(0xBB);
    println!(
        "Memory[0x{:04X}] = 0x{:02X}, Memory[0x{:04X}] = 0x{:02X}",
        start_addr * 2,
        vram.memory[start_addr as usize * 2],
        start_addr * 2 + 1,
        vram.memory[start_addr as usize * 2 + 1]
    );
    println!("VMADD after repeated write = 0x{:04X}", vram.vmadd);

    println!("\nNice and clean.");
}

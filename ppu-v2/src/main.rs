mod registers;
mod vram;
mod cgram;
mod constants;

use vram::VRAM;
use cgram::CGRAM;

fn main() {
    let mut vram = VRAM::new();
    println!("=== Testing VMAIN ===");

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
    // Test VMDATAL / VMDATAH
    // -----------------------------
    println!("\n--- Testing VMDATA write ---"); // ($2118/$2119)
    
    vram.write_vmain(0x80);
    
    // Write a word 0xABCD at current VMADD
    vram.write_vmdatal(0xCD);
    vram.write_vmdatah(0xAB);
    
    // Manualy check memory content (low byte / high byte)
    let mut addr = vram.byte_address() - 2; // previous address of the word written
    println!("Memory[0x{:04X}] (low1)  = 0x{:02X}", addr, vram.memory[addr]);
    println!("Memory[0x{:04X}] (high1) = 0x{:02X}", addr + 1, vram.memory[addr + 1]);
    println!("Memory[0x{:04X}] (low2)  = 0x{:02X}", addr + 2, vram.memory[addr + 2]);
    println!("Memory[0x{:04X}] (high2) = 0x{:02X}", addr + 3, vram.memory[addr + 3]);
    
    // -----------------------------
    println!("\n--- Testing VMDATA read ---"); // ($2139/$213A)

    println!("Low = 0x{:02X}", vram.read_vmdatal());
    println!("High = 0x{:02X}", vram.read_vmdatah());

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

    // -----------------------------
    // Test CGRAM
    // -----------------------------
    let mut cgram = CGRAM::new();
    println!("\n=== Testing CGRAM (hardware accurate) ===");

    // -------------------------------------------------
    // Test write colors
    // -------------------------------------------------

    cgram.write_addr(0);

    // Write first color 0x7C1F
    let color1 = 0x7C1F;
    cgram.write_data((color1 & 0xFF) as u8); // low
    cgram.write_data((color1 >> 8) as u8);   // high
    let stored1 = cgram.current_word(0);
    println!("CGRAM[0] expected 0x{:04X}, stored 0x{:04X}", color1, stored1);

    // Write second color 0x03E0
    let color2 = 0x03E0;
    cgram.write_data((color2 & 0xFF) as u8);
    cgram.write_data((color2 >> 8) as u8);
    let stored2 = cgram.current_word(1);
    println!("CGRAM[1] expected 0x{:04X}, stored 0x{:04X}", color2, stored2);

    // -------------------------------------------------
    // Test read colors
    // -------------------------------------------------

    cgram.write_addr(0);

    let low1 = cgram.read_data();
    let high1 = cgram.read_data();
    let read1 = ((high1 as u16) << 8) | low1 as u16;
    println!("Read CGRAM[0] = 0x{:04X}", read1);

    let low2 = cgram.read_data();
    let high2 = cgram.read_data();
    let read2 = ((high2 as u16) << 8) | low2 as u16;
    println!("Read CGRAM[1] = 0x{:04X}", read2);

    // -------------------------------------------------
    // Test address wrap (512 bytes)
    // -------------------------------------------------

    cgram.write_addr(255); // last word
    cgram.write_data(0xAA);
    cgram.write_data(0x55);

    // Next write should wrap to word 0
    cgram.write_data(0x11);
    cgram.write_data(0x22);

    let wrapped = cgram.current_word(0);
    println!("Wrap test: CGRAM[0] = 0x{:04X}", wrapped);

    // -------------------------------------------------
    // Test open bus behavior (bit 15)
    // -------------------------------------------------

    cgram.write_addr(0);
    let _ = cgram.read_data(); // low
    let high_with_open_bus = cgram.read_data();
    println!( "High byte with open bus bit applied: 0x{:02X}", high_with_open_bus );

    println!("\n>> Nice and clean.");
}

mod registers;
mod vram;
mod constants;

use vram::VRAM;

fn main() {
    let mut vram = VRAM::new();

    println!("--- VRAM Initial State ---");
    println!("VMADD: 0x{:04X}", vram.vmadd);
    println!("VMAIN: 0b{:08b}", vram.vmain);
    println!("VRAM latch: 0x{:04X}", vram.vram_latch);

    // --- Test VMAIN write ---
    vram.write_vmain(0b1000_0001);
    println!("\nAfter write_vmain(0b10000001):");
    println!("VMAIN: 0b{:08b}", vram.vmain);
    println!("Increment amount: {}", vram.increment_amount());
    println!(
        "- Increment after low byte: {}",
        vram.increment_after_low()
    );
    println!(
        "- Increment after high byte: {}",
        vram.increment_after_high()
    );

    // --- Test VMADD write ---
    vram.write_vmadd_low(0x34);
    vram.write_vmadd_high(0x12); // set VMADD = 0x1234
    println!("\nAfter setting VMADD to 0x1234:");
    println!("VMADD: 0x{:04X}", vram.vmadd);
    println!("Latch loaded: 0x{:04X}", vram.vram_latch);

    // --- Simulate changing memory to see latch update ---
    let addr = ((vram.vmadd & 0x7FFF) as usize) * 2;
    vram.memory[addr] = 0xAA;
    vram.memory[addr + 1] = 0x55;
    vram.load_latch();
    println!(
        "\nAfter writing 0x55AA at VMADD address in memory, latch: 0x{:04X}",
        vram.vram_latch
    );

    println!("\nNice and clean.");
}

// mod registers;
// mod vram;
// mod constants;

// use crate::vram::VRAM;

// fn main() {
//     let mut vram = VRAM::new();

//     // --- Test VMAIN ($2115) ---
//     vram.write_vmain(0b1000_0011);
//     println!("VMAIN: {:08b}", vram.vmain);

//     // ------ // ------ //
//     println!("\nNice and clean.");
// }
mod registers;
mod vram;
mod constants;

use crate::vram::VRAM;

fn main() {
    let mut vram = VRAM::new();

    // --- Test VMAIN ($2115) ---
    // Bits: M... RRII
    // II = 0b11 => increment amount = 128 words
    // M = 1 => increment after high byte write
    vram.write_vmain(0b1000_0011);
    println!("VMAIN: {:08b}", vram.vmain);

    // Vérifions l'auto-increment mode
    println!(
        "- Increment after low byte: {}",
        vram.increment_after_low()
    );
    println!(
        "- Increment after high byte: {}",
        vram.increment_after_high()
    );
    println!("- Increment amount (words): {}", vram.increment_amount());

    println!("\nNice and clean.");
}

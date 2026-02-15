mod registers;
mod vram;
mod constants;

use crate::vram::VRAM;

fn main() {
    let mut vram = VRAM::new();

    // --- Test VMAIN ($2115) ---
    vram.write_vmain(0b1000_0011);
    println!("VMAIN: {:08b}", vram.vmain);

    // ------ // ------ //
    println!("\nNice and clean.");
}

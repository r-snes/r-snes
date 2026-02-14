mod registers;

use crate::registers::PPURegisters;

fn main() {
    let mut ppu = PPURegisters::new();

    // --- Test INIDISP ($2100) ---
    // Forced blank = 1, brightness = 0x0F
    ppu.inidisp = 0b1000_1111;

    println!("INIDISP: {:08b}", ppu.inidisp);
    let forced_blank = (ppu.inidisp & 0b1000_0000) != 0;
    let brightness = ppu.inidisp & 0b0000_1111;
    println!("  Forced blank: {}", forced_blank);
    println!("  Brightness: {}", brightness);

    // --- Test BGMODE ($2105) ---
    // Mode 1, BG3 priority enabled
    ppu.bgmode = 0b0001_1001;

    println!("\nBGMODE: {:08b}", ppu.bgmode);
    let bg_mode = ppu.bgmode & 0b0000_0111;
    let bg3_priority = (ppu.bgmode & 0b0000_1000) != 0;
    println!("  BG Mode: {}", bg_mode);
    println!("  BG3 Priority: {}", bg3_priority);

    // --- Test OAM Address ($2102/$2103) ---
    ppu.oamaddl = 0x34;
    ppu.oamaddh = 0x01;

    let full_oam_addr = ((ppu.oamaddh as u16) << 8) | ppu.oamaddl as u16;
    println!("\nOAM Address: 0x{:04X}", full_oam_addr);

    // --- Test Scroll register ($210D BG1HOFS) ---
    ppu.bg1hofs = 0x01FF;
    println!("\nBG1 Horizontal Scroll: {}", ppu.bg1hofs);

    // --- Test CGDATA ($2122) ---
    ppu.cgdata = 0b01111_00000_11111; // Example SNES BGR555 color
    println!("\nCGRAM Write Value: {:016b}", ppu.cgdata);

    println!("\nAll tests executed successfully.");
}

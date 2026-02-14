mod registers;

use crate::registers::PPURegisters;

fn main() {
    let mut ppu = PPURegisters::new();

    // --- Test INIDISP ($2100) ---
    ppu.write(0x2100, 0b1000_1111);
    println!("INIDISP: {:08b}", ppu.inidisp);
    let forced_blank = (ppu.inidisp & 0b1000_0000) != 0;
    let brightness = ppu.inidisp & 0b0000_1111;
    println!("- Forced blank: {}", forced_blank);
    println!("- Brightness: {}", brightness);

    // --- Test BGMODE ($2105) ---
    ppu.write(0x2105, 0b0001_1001);
    println!("\nBGMODE: {:08b}", ppu.bgmode);
    let bg_mode = ppu.bgmode & 0b0000_0111;
    let bg3_priority = (ppu.bgmode & 0b0000_1000) != 0;
    println!("- BG Mode: {}", bg_mode);
    println!("- BG3 Priority: {}", bg3_priority);

    // --- Test OAM Address ($2102/$2103) ---
    ppu.write(0x2102, 0x34);
    ppu.write(0x2103, 0x01);
    let full_oam_addr = ((ppu.oamaddh as u16) << 8) | ppu.oamaddl as u16;
    println!("\nOAM Address: 0x{:04X}", full_oam_addr);

    // Test BG1HOFS ($210D) ---> 16-bit value write 
    ppu.write(0x210D, 0xFF); // low byte
    ppu.write(0x210D, 0x01); // high byte
    println!("\nBG1 Horizontal Scroll: {}", ppu.bg1hofs); // 0x01FF = 511

    // Test CGDATA ($2122) ---> 16-bit value write 
    let color: u16 = 0x3C1F; // BGR555
    ppu.write(0x2122, (color & 0xFF) as u8);       // low
    ppu.write(0x2122, ((color >> 8) & 0xFF) as u8); // high
    println!("\nCGRAM Write Value: {:016b}", ppu.cgdata); // 0011110000011111

    // ------ // ------ //
    println!("\nNice and clean.");
}

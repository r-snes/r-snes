mod ppu;

use ppu::PPU;

const VALUE: u8 = 0xAB;

fn main() {
    let mut ppu = PPU::new();

    ppu.write_vram(0x1234, VALUE);

    let res = ppu.read_vram(0x1234);

    assert_eq!(res, VALUE);
    println!("All good :)");
}

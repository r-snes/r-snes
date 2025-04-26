mod ppu;
mod tile;

use ppu::PPU;
use tile::{ TILE_SIZE, load_and_split_image };

fn main() {
    let mut ppu = PPU::new();

    let tiles = load_and_split_image("./tests/assets/a.jpg");

    // checking the 4 first tiles
    for i in 0..4 {
        let tile = &tiles[i];

        // tmp -> using red as the palette index
        for (j, pixel) in tile.iter().enumerate() {
            ppu.write_vram(i * TILE_SIZE as usize + j, pixel[0]);
        }
    }

    for i in 0..4 {
        println!("Tile {}: ", i);
        for y in 0..TILE_SIZE as usize {
            for x in 0..TILE_SIZE as usize {
                let addr = i * TILE_SIZE as usize + y * TILE_SIZE as usize + x;
                print!("{:02X} ", ppu.read_vram(addr)); // hexa display
            }
            println!();
        }
    }

    println!("All good :)");
}

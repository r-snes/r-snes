use ppu::constants::*;
use ppu::ppu::PPU;
use ppu::rendering::renderer::Renderer;

use sdl2::pixels::PixelFormatEnum;

fn main() {
    let mut ppu = PPU::new();
    let mut renderer = Renderer::new();

    // ============================================================
    // CGRAM: test gradient (BG colors) + one bright sprite color
    // ============================================================
    for i in 0u8..=255 {
        ppu.write(0x2121, i);
        ppu.write(0x2122, i);
        ppu.write(0x2122, 0x00);
    }

    // Sprite palette 0, color index 1 -> CGRAM entry 128 + 0*16 + 1 = 129 (red).
    ppu.write(0x2121, 129);
    ppu.write(0x2122, 0x1F); // low byte  (R = 31)
    ppu.write(0x2122, 0x00); // high byte -> CGRAM[129] = 0x001F (red)

    // Sprite palette 1, color index 1 -> CGRAM entry 128 + 1*16 + 1 = 145 (green).
    ppu.write(0x2121, 145);
    ppu.write(0x2122, 0xE0); // low byte  (G low bits)
    ppu.write(0x2122, 0x03); // high byte -> CGRAM[145] = 0x03E0 (green)

    // ============================================================
    // VRAM setup
    // ============================================================
    ppu.write(0x2115, 0x80); // VMAIN: increment after high byte, step 1

    // ---- BG1 tiles (16 color bands), CHR at word 0 ----
    for tile in 0u16..16 {
        let tile_word_base = tile * 16; // 32 bytes = 16 words per tile

        for row in 0u16..8 {
            let word_addr = tile_word_base + row;
            ppu.write(0x2116, (word_addr & 0xFF) as u8);
            ppu.write(0x2117, (word_addr >> 8) as u8);

            let p0_low: u8 = if tile & 1 != 0 { 0xFF } else { 0x00 };
            let p0_high: u8 = if tile & 2 != 0 { 0xFF } else { 0x00 };

            ppu.write(0x2118, p0_low);
            ppu.write(0x2119, p0_high);
        }

        for row in 0u16..8 {
            let word_addr = tile_word_base + 8 + row;
            ppu.write(0x2116, (word_addr & 0xFF) as u8);
            ppu.write(0x2117, (word_addr >> 8) as u8);

            let p1_low: u8 = if tile & 4 != 0 { 0xFF } else { 0x00 };
            let p1_high: u8 = if tile & 8 != 0 { 0xFF } else { 0x00 };

            ppu.write(0x2118, p1_low);
            ppu.write(0x2119, p1_high);
        }
    }

    // ---- BG1 tilemap at word 0x0400 (16 vertical color bands) ----
    let tilemap_word_base: u16 = 0x0400;
    for row in 0u16..32 {
        for col in 0u16..32 {
            let word_addr = tilemap_word_base + row * 32 + col;
            ppu.write(0x2116, (word_addr & 0xFF) as u8);
            ppu.write(0x2117, (word_addr >> 8) as u8);

            let tile_index: u16 = (col / 2) % 16;
            ppu.write(0x2118, (tile_index & 0xFF) as u8);
            ppu.write(0x2119, ((tile_index >> 8) & 0x03) as u8);
        }
    }

    // ---- Sprite CHR ----
    // OBJSEL name base 1 -> sprite CHR base = 1 << 13 = word 0x2000.
    // Each 4bpp tile is 16 words; a tile at tile-number N sits at 0x2000 + N*16.
    // We fill a set of tiles with a solid color-index-1 pattern (plane 0 all set).
    //  - tile 0        -> the 8x8 sprite
    //  - tiles 2,3,0x12,0x13 -> the four quadrants of the 16x16 sprite
    let sprite_chr_base: u16 = 0x2000;
    let solid_tiles: [u16; 5] = [0, 2, 3, 0x12, 0x13];
    for &tile_num in solid_tiles.iter() {
        let base = sprite_chr_base + tile_num * 16;
        ppu.write(0x2116, (base & 0xFF) as u8);
        ppu.write(0x2117, (base >> 8) as u8);
        for _row in 0..8 {
            ppu.write(0x2118, 0xFF); // plane 0 (low)  -> color index 1
            ppu.write(0x2119, 0x00); // plane 1 (high)
        }
        for _row in 0..8 {
            ppu.write(0x2118, 0x00); // plane 2
            ppu.write(0x2119, 0x00); // plane 3
        }
    }

    // ============================================================
    // OAM setup
    // ============================================================
    // Park all 128 sprites off-screen (Y = 0xE0) so the zeroed OAM doesn't
    // draw a block of tile-0 sprites at the top-left corner.
    // OAMADD is a WORD address: sprite i's table-1 entry is word i*2.
    for i in 0u16..128 {
        let word = i * 2;
        ppu.write(0x2102, (word & 0xFF) as u8);
        ppu.write(0x2103, ((word >> 8) & 0x01) as u8);
        ppu.write(0x2104, 0x00); // X lo -> latched
        ppu.write(0x2104, 0xE0); // Y    -> commits (X=0, Y=0xE0)
    }

    // Sprite 0: 8x8 red sprite (word address 0).
    // Table 1: X=100, Y=108, tile=0, attr = priority 2 (0x20), palette 0.
    ppu.write(0x2102, 0x00);
    ppu.write(0x2103, 0x00);
    ppu.write(0x2104, 100);  // X    -> latched
    ppu.write(0x2104, 108);  // Y    -> commits word (X, Y)
    ppu.write(0x2104, 0x00); // tile -> latched
    ppu.write(0x2104, 0x20); // attr = priority 2, palette 0 (red)

    // Sprite 1: 16x16 green sprite (word address 2 = sprite 1's table-1 entry).
    // Table 1: X=140, Y=104, tile=2, attr = priority 2, palette 1.
    // attr layout vhoopppN: priority 2 = 0x20, palette 1 = (1 << 1) = 0x02 -> 0x22.
    ppu.write(0x2102, 0x02);
    ppu.write(0x2103, 0x00);
    ppu.write(0x2104, 140);  // X    -> latched
    ppu.write(0x2104, 104);  // Y    -> commits word (X, Y)
    ppu.write(0x2104, 0x02); // tile -> latched (tile 2)
    ppu.write(0x2104, 0x22); // attr = priority 2, palette 1 (green)

    // Sprite 1's large bit lives in OAM table 2. Sprite 1 occupies bits 3:2 of
    // the first table-2 byte (byte 512, word address 256). large = bit 3.
    // Now reachable thanks to word-based addressing.
    ppu.write(0x2102, 0x00);
    ppu.write(0x2103, 0x01); // word 256 -> byte 512 (table 2 start)
    ppu.write(0x2104, 0b0000_1000); // sprite1 large = 1 (bits 3:2), sprite0 = 00

    // ============================================================
    // PPU registers
    // ============================================================
    ppu.write(0x2100, 0x0F); // INIDISP: display on, full brightness
    ppu.write(0x2101, 0x01); // OBJSEL:  size mode 0 (small 8x8 / large 16x16), CHR base 0x2000
    ppu.write(0x2105, 0x01); // BGMODE:  mode 1
    ppu.write(0x2107, 0x04); // BG1SC:   tilemap at word 0x0400, 32x32
    ppu.write(0x212C, 0x11); // TM:      BG1 (0x01) + OBJ (0x10) enabled
    // ppu.write(0x212C, 0x01); // TM:      BG1 only (OBJ disabled)

    // ============================================================
    // SDL2
    // ============================================================
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    let window = video
        .window("SNES PPU - sprite test", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            SCREEN_WIDTH as u32,
            SCREEN_HEIGHT as u32,
        )
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    for y in 0..SCREEN_HEIGHT {
        renderer.render_scanline(&ppu, y);
    }
    renderer.swap_buffers();

    'running: loop {
        for event in event_pump.poll_iter() {
            if let sdl2::event::Event::Quit { .. } = event {
                break 'running;
            }
        }

        texture
            .update(None, &renderer.presented()[..], SCREEN_WIDTH * 3)
            .unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    println!("\n>> Nice and clean.");
}

use ppu::constants::*;
use ppu::ppu::PPU;
use ppu::renderer::Renderer;

use sdl2::pixels::PixelFormatEnum;

fn write_text(ppu: &mut PPU, y: u16, text: &[u8]) {
    ppu.write(0x2116, (y & 0xFF) as u8);
    ppu.write(0x2117, (y >> 8) as u8);

    for c in text {
        ppu.write(0x2118, *c);
        ppu.write(0x2119, 0);
    }
}

fn main() {
    let mut ppu = PPU::new();
    let mut renderer = Renderer::new();

    // Fill CGRAM with a colour which should be unused
    for i in 0u8..=255 {
        ppu.write(0x2121, i);
        ppu.write(0x2122, 0x70);
        ppu.write(0x2122, 0xC7);
    }

    // == Init regs: various PPU regs + CGRAM palette
    ppu.write(0x2100, 0x8F); // screen off
    ppu.write(0x2105, 0);
    ppu.write(0x2106, 0);
    ppu.write(0x2107, 0);
    ppu.write(0x210B, 0x04);
    ppu.write(0x210D, 0);
    ppu.write(0x210D, 0); // write twice
    ppu.write(0x210E, 0xFF);
    ppu.write(0x210E, 0xFF); // write twice
    ppu.write(0x2115, 0x80); // VMAIN

    ppu.write(0x2121, 0); // CGADDR
    ppu.write(0x2122, 0);
    ppu.write(0x2122, 0); // write twice black in palette 0
    ppu.write(0x2122, 0xFF);
    ppu.write(0x2122, 0xFF); // write twice white in palette 1 (auto inc)

    ppu.write(0x212C, 1); // BG1
    ppu.write(0x212D, 0);
    ppu.write(0x212E, 0);
    ppu.write(0x2130, 0);
    ppu.write(0x2131, 0x30);
    ppu.write(0x2133, 0);

    // completely zero out all the VRAM
    ppu.write(0x2116, 0);
    ppu.write(0x2117, 0); // set VADDR to 0
    for _ in 0..=0x7FFF {
        ppu.write(0x2118, 0);
        ppu.write(0x2119, 0);
    }

    // load font to VRAM
    ppu.write(0x2116, 0);
    ppu.write(0x2117, 0x40); // set VADDR to 0x4000
    assert_eq!(ppu.vram.vmadd, 0x4000);
    let font: &[u8; 2048] = include_bytes!("../font.bin");
    for i in 0..1024 {
        ppu.write(0x2118, font[i*2]);
        ppu.write(0x2119, font[i*2 + 1]);
    }
    assert_eq!(font, &ppu.vram.memory[0x8000..0x8800]);

    write_text(&mut ppu, 0x32, b"this is text");

    ppu.write(0x2100, 0x0F); // screen on

    assert_eq!(ppu.regs.bg1_tilemap_addr(), 0);
    assert_eq!(ppu.regs.bg1_tiledata_addr(), 0x4000);

    // for tile in 0u16..16 {
    //     let tile_word_base = tile * 16; // 32 bytes = 16 words per tile
    //
    //     // Plane 0: low/high bitplane
    //     for row in 0u16..8 {
    //         let word_addr = tile_word_base + row;
    //         ppu.write(0x2116, (word_addr & 0xFF) as u8);
    //         ppu.write(0x2117, (word_addr >> 8) as u8);
    //
    //         let p0_low: u8 = if tile & 1 != 0 { 0xFF } else { 0x00 };
    //         let p0_high: u8 = if tile & 2 != 0 { 0xFF } else { 0x00 };
    //
    //         ppu.write(0x2118, p0_low);  // VMDATAL
    //         ppu.write(0x2119, p0_high); // VMDATAH
    //     }
    //
    //     // Plane 1: offset +8 words
    //     for row in 0u16..8 {
    //         let word_addr = tile_word_base + 8 + row;
    //         ppu.write(0x2116,  (word_addr & 0xFF) as u8);
    //         ppu.write(0x2117, (word_addr >> 8) as u8);
    //
    //         let p1_low: u8 = if tile & 4 != 0 { 0xFF } else { 0x00 };
    //         let p1_high: u8 = if tile & 8 != 0 { 0xFF } else { 0x00 };
    //
    //         ppu.write(0x2118, p1_low);
    //         ppu.write(0x2119, p1_high);
    //     }
    // }
    //
    // let tilemap_word_base: u16 = 0x0400;
    //
    // for row in 0u16..32 {
    //     for col in 0u16..32 {
    //         let word_addr = tilemap_word_base + row * 32 + col;
    //         ppu.write(0x2116, (word_addr & 0xFF) as u8);
    //         ppu.write(0x2117, (word_addr >> 8) as u8);
    //
    //         // 2 tile columns per color band, 16 bands total
    //         let tile_index: u16 = (col / 2) % 16;
    //
    //         let entry_low  = (tile_index & 0xFF) as u8;
    //         let entry_high = ((tile_index >> 8) & 0x03) as u8;
    //
    //         ppu.write(0x2118, entry_low);
    //         ppu.write(0x2119, entry_high);
    //     }
    // }

    // // PPU registers
    // ppu.write(0x2100, 0x0F); // INIDISP (display on, max brightness)
    // ppu.write(0x2105, 0x01); // BGMODE (Mode 1)
    // ppu.write(0x2107, 0x04); // BG1SC (tilemap -> word 0x0400, 32x32)
    // ppu.write(0x212C, 0x01); // TM (BG1 enabled)

    // SDL2 initialization
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    let window = video
        .window("SNES PPU", SCREEN_WIDTH as u32 * 3, SCREEN_HEIGHT as u32 * 3)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            if let sdl2::event::Event::Quit { .. } = event {
                break 'running;
            }
        }

        for y in 0..SCREEN_HEIGHT {
            renderer.render_scanline(&ppu, y);
            ppu.step_scanline();
        }

        texture.update(None, &renderer.framebuffer, SCREEN_WIDTH * 3).unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        ppu.frame_ready = false;
    }
    println!("\n>> Nice and clean.");
}

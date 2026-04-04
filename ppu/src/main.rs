mod constants;
mod vram;
mod cgram;
mod ppu;
mod registers;
mod renderer;

use constants::*;
use ppu::PPU;
use renderer::Renderer;

use sdl2::pixels::PixelFormatEnum;

fn main() {
    let mut ppu = PPU::new();
    let mut renderer = Renderer::new();

    // Fill CGRAM with test gradient
    for i in 0u8..=255 {
        ppu.write(0x2121, i);
        ppu.write(0x2122, i);
        ppu.write(0x2122, 0x00);
    }

    // Fill VRAM
    ppu.write(0x2115, 0x80);

    for tile in 0u16..16 {
        let tile_word_base = tile * 16; // 32 bytes = 16 words per tile

        // Plane 0: low/high bitplane
        for row in 0u16..8 {
            let word_addr = tile_word_base + row;
            ppu.write(0x2116, (word_addr & 0xFF) as u8);
            ppu.write(0x2117, (word_addr >> 8) as u8);

            let p0_low: u8 = if tile & 1 != 0 { 0xFF } else { 0x00 };
            let p0_high: u8 = if tile & 2 != 0 { 0xFF } else { 0x00 };

            ppu.write(0x2118, p0_low);  // VMDATAL
            ppu.write(0x2119, p0_high); // VMDATAH
        }

        // Plane 1: offset +8 words
        for row in 0u16..8 {
            let word_addr = tile_word_base + 8 + row;
            ppu.write(0x2116,  (word_addr & 0xFF) as u8);
            ppu.write(0x2117, (word_addr >> 8) as u8);

            let p1_low: u8 = if tile & 4 != 0 { 0xFF } else { 0x00 };
            let p1_high: u8 = if tile & 8 != 0 { 0xFF } else { 0x00 };

            ppu.write(0x2118, p1_low);
            ppu.write(0x2119, p1_high);
        }
    }

    let tilemap_word_base: u16 = 0x0400;

    for row in 0u16..32 {
        for col in 0u16..32 {
            let word_addr = tilemap_word_base + row * 32 + col;
            ppu.write(0x2116, (word_addr & 0xFF) as u8);
            ppu.write(0x2117, (word_addr >> 8) as u8);

            // 2 tile columns per color band, 16 bands total
            let tile_index: u16 = (col / 2) % 16;

            let entry_low  = (tile_index & 0xFF) as u8;
            let entry_high = ((tile_index >> 8) & 0x03) as u8;

            ppu.write(0x2118, entry_low);
            ppu.write(0x2119, entry_high);
        }
    }

    // PPU registers
    ppu.write(0x2100, 0x0F); // INIDISP (display on, max brightness)
    ppu.write(0x2105, 0x01); // BGMODE (Mode 1)
    ppu.write(0x2107, 0x04); // BG1SC (tilemap -> word 0x0400, 32x32)
    ppu.write(0x212C, 0x01); // TM (BG1 enabled)

    // SDL2 initialization
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    let window = video
        .window("SNES PPU", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
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

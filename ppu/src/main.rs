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
    for i in 0..256 {
        let i = i as u8;

        // $2121 - CGADD
        ppu.write(0x2121, i);

        // $2122 - CGDATA
        ppu.write(0x2122, i); // low byte
        ppu.write(0x2122, 0x00); // high byte
    }

    ppu.write(0x2100, 0x0F); // Enable display
    ppu.write(0x2105, 0x01); // Mode 1
    ppu.write(0x212C, 0x01); // Enable BG1

    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    let window = video
        .window("SNES PPU Scanline", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
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

        // Render full frame
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

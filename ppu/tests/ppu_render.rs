use ppu::ppu::PPU;
use ppu::utils::{render_scanline, WIDTH, HEIGHT, TILE_SIZE};

// Helper: fill a single tile in VRAM with a given color
fn fill_tile(ppu: &mut PPU, tile_index: usize, color: u8) {
    let base = tile_index * TILE_SIZE * TILE_SIZE;
    for i in 0..(TILE_SIZE * TILE_SIZE) {
        ppu.write_vram(base + i, color);
    }
}

#[test] // Should fill the top-left tile and reflect it in the top-left of the framebuffer
fn test_render_draws_tile_at_correct_position() {
    let mut ppu = PPU::new();
    fill_tile(&mut ppu, 0, 0xFF);

    ppu.render(WIDTH / TILE_SIZE);

    // Top-left tile occupies pixels (0..8, 0..8)
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let index = y * WIDTH + x;
            assert_ne!(ppu.framebuffer[index], 0, "Pixel at ({}, {}) should not be 0", x, y);
        }
    }
}

#[test] // Should not draw anything if scanline is out of bounds
fn test_render_scanline_out_of_bounds_does_nothing() {
    let mut ppu = PPU::new();

    ppu.framebuffer.iter_mut().for_each(|px| *px = 42);

    render_scanline(&mut ppu, HEIGHT + 10, WIDTH / TILE_SIZE);

    assert!(ppu.framebuffer.iter().all(|&px| px == 42));
}

#[test] // Should only modify the relevant scanline
fn test_render_scanline_modifies_only_one_line() {
    let mut ppu = PPU::new();
    fill_tile(&mut ppu, 0, 0xFF);

    let scanline_y = 3;

    // Render a single scanline
    render_scanline(&mut ppu, scanline_y, WIDTH / TILE_SIZE);

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let index = y * WIDTH + x;
            if y == scanline_y {
                assert_ne!(ppu.framebuffer[index], 0, "Pixel at ({}, {}) should be written", x, y);
            } else {
                assert_eq!(ppu.framebuffer[index], 0, "Pixel at ({}, {}) should be untouched", x, y);
            }
        }
    }
}

#[test] // Should fully redraw the screen with repeating tiles
fn test_render_full_screen_repeat_tile() {
    let mut ppu = PPU::new();
    fill_tile(&mut ppu, 0, 0xFF);

    ppu.render(WIDTH / TILE_SIZE);

    let filled = ppu.framebuffer.iter().filter(|&&px| px != 0).count();
    assert!(filled > (0.8 * (WIDTH * HEIGHT) as f64) as usize, "Most of the screen should be filled");
}

#[test] // Should render a scanline from tile 0 with the expected color
fn test_render_scanline_renders_correct_line() {
    let mut ppu = PPU::new();
    let tile_index = 0;

    fill_tile(&mut ppu, tile_index, 0xFF);

    render_scanline(&mut ppu, 0, WIDTH / TILE_SIZE);

    for x in 0..TILE_SIZE {
        let color = ppu.framebuffer[x];
        println!("----Actual color: {:#X}", color);
        assert_eq!(color, 0xFFFFFFF0);
    }
}

use crate::ppu::*;
use crate::utils::{render_scanline, WIDTH, HEIGHT, TILE_SIZE, OAM_MAX_SPRITES};

#[test] // OAM should initialize with 128 empty sprite entries
fn test_oam_initialization() {
    let ppu = PPU::new();
    assert_eq!(ppu.oam.len(), OAM_MAX_SPRITES);
    for spr in &ppu.oam {
        assert_eq!(spr.x, 0);
        assert_eq!(spr.y, 0);
        assert_eq!(spr.tile, 0);
        assert_eq!(spr.attr, 0);
    }
}

#[test] // Writing a sprite should store its attributes
fn test_write_oam_entry() {
    let mut ppu = PPU::new();
    let sprite = Sprite { x: 10, y: 20, tile: 3, attr: 0xC0, filed: true };
    ppu.set_oam_sprite(0, sprite);
    let stored = ppu.get_oam_sprite(0).unwrap();
    assert_eq!(stored.x, 10);
    assert_eq!(stored.y, 20);
    assert_eq!(stored.tile, 3);
    assert_eq!(stored.attr, 0xC0);
}

#[test] // Getting an OAM sprite out of range should return None
fn test_get_oam_out_of_range() {
    let ppu = PPU::new();
    assert!(ppu.get_oam_sprite(OAM_MAX_SPRITES).is_none());
}

#[test] // Setting an OAM sprite out of range should do nothing
fn test_set_oam_out_of_range() {
    let mut ppu = PPU::new();
    let sprite = Sprite { x: 1, y: 1, tile: 1, attr: 0, filed: true };
    ppu.set_oam_sprite(OAM_MAX_SPRITES, sprite);
    assert_eq!(ppu.oam[OAM_MAX_SPRITES-1].x, 0);
}

#[test] // Flip flags should be preserved
fn test_sprite_flip_flags() {
    let mut ppu = PPU::new();
    let spr = Sprite { x:0, y:0, tile:0, attr: 0xC0, filed: true };
    ppu.set_oam_sprite(0, spr);
    let read = ppu.get_oam_sprite(0).unwrap();
    assert_eq!(read.attr & 0x40, 0x40); // hflip
    assert_eq!(read.attr & 0x80, 0x80); // vflip
}

#[test] // Clear framebuffer before rendering scanline
fn test_render_scanline_clears() {
    let mut ppu = PPU::new();
    for i in 0..WIDTH*HEIGHT {
        ppu.framebuffer[i] = 0x12345678;
    }
    render_scanline(&mut ppu, 0);
    for x in 0..WIDTH {
        assert_eq!(ppu.framebuffer[x], 0xFF000000);
    }
}

#[test] // Rendering a single sprite at 0,0 should color first pixels
fn test_render_single_sprite() {
    let mut ppu = PPU::new();
    // fill VRAM tile 0 with palette index 1
    for i in 0..TILE_SIZE*TILE_SIZE {
        ppu.write_vram(i, 1);
    }
    let sprite = Sprite { x:0, y:0, tile:0, attr:0, filed: true };
    ppu.set_oam_sprite(0, sprite);

    render_scanline(&mut ppu, 0);
    // framebuffer[0] should not be black (palette index 0 = transparent)
    assert_ne!(ppu.framebuffer[0], 0xFF000000);
}

#[test] // Transparency: palette index 0 should not render
fn test_sprite_transparency() {
    let mut ppu = PPU::new();
    for i in 0..TILE_SIZE*TILE_SIZE {
        ppu.write_vram(i, 0); // all transparent
    }
    let sprite = Sprite { x:0, y:0, tile:0, attr:0, filed: true };
    ppu.set_oam_sprite(0, sprite);
    render_scanline(&mut ppu, 0);
    // framebuffer should remain black
    assert_eq!(ppu.framebuffer[0], 0xFF000000);
}

#[test] // Sprite horizontal flip works
fn test_sprite_hflip() {
    let mut ppu = PPU::new();
    for i in 0..TILE_SIZE*TILE_SIZE {
        ppu.write_vram(i, (i % 2 + 1) as u8); // pattern 1,2,1,2...
    }
    let sprite = Sprite { x:0, y:0, tile:0, attr:0x40, filed: true }; // hflip
    ppu.set_oam_sprite(0, sprite);
    render_scanline(&mut ppu, 0);
    let left = ppu.framebuffer[0];
    let right = ppu.framebuffer[TILE_SIZE-1];
    assert_ne!(left, right);
}

#[test] // Sprite vertical flip works
fn test_sprite_vflip() {
    let mut ppu = PPU::new();
    for i in 0..TILE_SIZE*TILE_SIZE {
        ppu.write_vram(i, (i % 2 + 1) as u8); 
    }
    let sprite = Sprite { x:0, y:0, tile:0, attr:0x80, filed: true }; // vflip
    ppu.set_oam_sprite(0, sprite);
    render_scanline(&mut ppu, TILE_SIZE-1);
    let last = ppu.framebuffer[(TILE_SIZE-1) * WIDTH];
    assert_ne!(last, 0xFF000000);
}

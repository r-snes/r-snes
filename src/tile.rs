use image::{GenericImageView, Rgba, Pixel};

pub const TILE_SIZE: u32 = 8;

pub fn load_and_split_image(image_path: &str) -> Vec<Vec<Rgba<u8>>> {
    let img = image::open(image_path).expect("Failed to open image");
    let (width, height) = img.dimensions();
    let mut tiles = Vec::new();

    for y in (0..height).step_by(TILE_SIZE as usize) {
        for x in (0..width).step_by(TILE_SIZE as usize) {
            let mut tile = Vec::new();

            for dy in 0..TILE_SIZE {
                for dx in 0..TILE_SIZE {
                    let px = x + dx;
                    let py = y + dy;
                    if px < width && py < height {
                        let pixel = img.get_pixel(px, py);
                        tile.push(pixel.to_rgba());
                    }
                }
            }

            tiles.push(tile);
        }
    }

    tiles
}

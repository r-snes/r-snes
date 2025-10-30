use crate::ppu::PPU;
use crate::constants::*;

/// Retrieves the palette indices of a tile from VRAM
///
/// This function reads TILE_SIZE x TILE_SIZE bytes from VRAM starting at the tile's base address,
/// returning the raw palette indices without converting them to colors
///
/// # Parameters
/// - `ppu`: Reference to the PPU, which contains VRAM
/// - `tile_index`: The index of the tile in VRAM
///
/// # Returns
/// A `Vec<u8>` containing the palette indices for each pixel in row-major order
pub fn get_tile_indices_from_vram(ppu: &PPU, tile_index: usize) -> Vec<u8> {
    let mut indices = Vec::with_capacity(TILE_SIZE * TILE_SIZE);
    let base_addr = tile_index * TILE_SIZE * TILE_SIZE;

    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let addr = base_addr + y * TILE_SIZE + x;
            indices.push(ppu.read_vram(addr));
        }
    }

    indices
}

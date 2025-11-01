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

#[cfg(test)]
mod tests_tile {
    use super::*;
    use crate::ppu::PPU;

    #[test] // Retrieves palette indices from VRAM for the first tile
    fn test_get_tile_indices_from_vram_returns_indices() {
        let mut ppu = PPU::new();

        // Fill first tile region in VRAM with a pattern
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.write_vram(i, (i % 64) as u8);
        }

        let indices = get_tile_indices_from_vram(&ppu, 0);
        assert_eq!(indices.len(), TILE_SIZE * TILE_SIZE);
        assert_eq!(indices[0], 0);
        assert_eq!(indices[1], 1);
        assert_eq!(indices[7], 7);
    }

    #[test] // Should correctly read from multiple tiles
    fn test_get_tile_indices_from_vram_multiple_tiles() {
        let mut ppu = PPU::new();

        // Fill VRAM with two different tiles:
        // Tile 0 => filled with 1s
        // Tile 1 => filled with 2s
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.write_vram(i, 1);
            ppu.write_vram(i + TILE_SIZE * TILE_SIZE, 2);
        }

        let tile0 = get_tile_indices_from_vram(&ppu, 0);
        let tile1 = get_tile_indices_from_vram(&ppu, 1);

        assert!(tile0.iter().all(|&v| v == 1));
        assert!(tile1.iter().all(|&v| v == 2));
    }

    #[test] // Reading a tile that starts near the end of VRAM should not panic
    fn test_get_tile_indices_from_vram_out_of_bounds_safe() {
        let mut ppu = PPU::new();

        // Fill VRAM partially with known pattern
        for i in 0..VRAM_SIZE.min(256) {
            ppu.write_vram(i, (i % 255) as u8);
        }

        // Pick a tile index that goes beyond VRAM
        let tile_index = VRAM_SIZE / (TILE_SIZE * TILE_SIZE) + 1;

        // Should return a tile vector without panicking
        let indices = get_tile_indices_from_vram(&ppu, tile_index);

        // Vector should have the correct size
        assert_eq!(indices.len(), TILE_SIZE * TILE_SIZE);
    }

    #[test] // All indices must be read in row-major order
    fn test_get_tile_indices_row_major_order() {
        let mut ppu = PPU::new();

        // Fill VRAM with incrementing values
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.write_vram(i, i as u8);
        }

        let indices = get_tile_indices_from_vram(&ppu, 0);

        // The tile should be sequential in memory (row-major)
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            assert_eq!(indices[i], i as u8);
        }
    }

    #[test] // Check that function behaves consistently across multiple calls
    fn test_get_tile_indices_deterministic() {
        let mut ppu = PPU::new();

        for i in 0..(TILE_SIZE * TILE_SIZE) {
            ppu.write_vram(i, (i * 3 % 256) as u8);
        }

        let first = get_tile_indices_from_vram(&ppu, 0);
        let second = get_tile_indices_from_vram(&ppu, 0);

        assert_eq!(first, second);
    }
}

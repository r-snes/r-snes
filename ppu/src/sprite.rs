/// Represents a single sprite in the PPU's OAM.
///
/// Each sprite has a position (`x`, `y`), a tile index pointing into VRAM,
/// attribute flags (`attr`) for palette selection and flipping, and a `filed`
/// boolean indicating whether the sprite is active or not
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    pub x: i16,
    pub y: i16,
    pub tile: u16,
    pub attr: u8,
    pub filed: bool,
}

/// Provides a default sprite instance
///
/// Returns a sprite with all fields initialized to zero or false
/// Useful for initializing OAM arrays with empty/default sprites
impl Default for Sprite {
    fn default() -> Self {
        Sprite { x: 0, y: 0, tile: 0, attr: 0, filed: false }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub x: i16,
    pub y: i16,
    pub tile: u16,
    pub attr: u8,
    pub filed: bool,
}

impl Default for Sprite {
    fn default() -> Self {
        Sprite { x: 0, y: 0, tile: 0, attr: 0, filed: false }
    }
}

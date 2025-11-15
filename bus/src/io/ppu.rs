pub use crate::io::Io;
use common::snes_address::SnesAddress;

impl Io {
    pub fn handle_ppu_read(addr: SnesAddress) -> u8 {
        0
    }

    pub fn handle_ppu_write(addr: SnesAddress, value: u8) {}
}

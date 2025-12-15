/// Represents the Video RAM (VRAM).
///
/// VRAM is responsible for storing graphical data such as tiles, maps
/// and other video-related memory. Access to this memory is indirect and goes through
/// address registers, data ports, internal latches, and a buffered read mechanism.
///
/// This structure encapsulates the memory storage itself as well as all the state required
/// to manage sequential reads, writes, and automatic address advancement.
#[derive(Debug)]
pub struct Vram {
    mem: Vec<u8>,
    vma_addr: u32,
    addr_latch_low: Option<u8>,
    read_buffer: u16,
    read_buffer_valid: bool,
    read_byte_index: u8, // 0 -> next read returns: 0 = low byte, 1 = high byte
    auto_inc: u16,
}

impl Vram {

    /// Creates a new VRAM instance initialized to power-on state.
    ///
    /// VRAM is filled with zeros, the VMA address is reset to 0,
    /// read/write latches are cleared, and auto-increment is set to 1.
    pub fn new() -> Self {
        Self {
            mem: vec![0; 0x10000],
            vma_addr: 0,
            addr_latch_low: None,
            read_buffer: 0,
            read_buffer_valid: false,
            read_byte_index: 0,
            auto_inc: 1,
        }
    }

    /// Loads raw data into VRAM memory starting at address 0.
    ///
    /// This function copies the provided slice into the internal VRAM buffer.
    /// If the source slice is larger than the VRAM size, only the first 64 KiB are copied.
    /// Existing contents are overwritten.
    ///
    /// This is mainly intended for initialization, testing, or debugging.
    pub fn load_from_slice(&mut self, src: &[u8]) {
        let len = usize::min(src.len(), 0x10000);
        self.mem[..len].copy_from_slice(&src[..len]);
    }

    /// Writes the low byte of the current VRAM address.
    ///
    /// This updates only the lower 8 bits of the internal address while keeping
    /// the previously written high byte intact. Any pending read buffer state is cleared,
    /// ensuring that subsequent reads start a new buffered sequence.
    pub fn write_addr_low(&mut self, value: u8) {
        let high = ((self.vma_addr >> 8) & 0xFF) as u8;
        let new = (((high as u16) << 8) | (value as u16)) as u32;
        self.vma_addr = new & 0xFFFF;
        self.read_buffer_valid = false;
        self.read_byte_index = 0;
    }

    /// Writes the high byte of the current VRAM address.
    ///
    /// This updates only the upper 8 bits of the internal address while keeping
    /// the previously written low byte intact. Any pending read buffer state is cleared,
    /// ensuring that subsequent reads start a new buffered sequence.
    pub fn write_addr_high(&mut self, value: u8) {
        let low = (self.vma_addr & 0xFF) as u8;
        let new = ((((value as u16) << 8) | (low as u16)) as u32) & 0xFFFF;
        self.vma_addr = new;
        self.read_buffer_valid = false;
        self.read_byte_index = 0;
    }

    /// Directly sets the current VRAM address using a full 16-bit value.
    ///
    /// This bypasses the low/high write sequence and replaces the address entirely.
    /// Any buffered read state is cleared so that subsequent reads behave
    /// as if the address has just been changed.
    pub fn set_addr(&mut self, addr: u16) {
        self.vma_addr = (addr as u32) & 0xFFFF;
        self.read_buffer_valid = false;
        self.read_byte_index = 0;
    }

    /// Returns the current VRAM address.
    ///
    /// Only the lower 16 bits are exposed, as the address space is limited to 64 KiB.
    pub fn get_addr(&self) -> u16 {
        (self.vma_addr & 0xFFFF) as u16
    }

    /// Sets the automatic address increment value.
    ///
    /// This value is added to the current address after each completed read or write operation,
    /// allowing sequential access patterns without manually updating the address each time.
    pub fn set_auto_increment(&mut self, inc: u16) {
        self.auto_inc = inc;
    }

    /// Writes a 16-bit word directly into VRAM memory at the given address.
    ///
    /// The word is stored in little-endian order:
    /// - low byte at `addr`
    /// - high byte at `addr + 1`
    ///
    /// Address wrapping is handled automatically within the 64 KiB space.
    pub fn mem_write16_at(&mut self, addr: u16, word: u16) {
        let a = addr as usize;
        let lo = (word & 0xFF) as u8;
        let hi = (word >> 8) as u8;
        self.mem[a % 0x10000] = lo;
        self.mem[(a.wrapping_add(1)) % 0x10000] = hi;
    }

    /// Reads a 16-bit word directly from VRAM memory at the given address.
    ///
    /// The value is reconstructed assuming little-endian layout:
    /// - low byte from `addr`
    /// - high byte from `addr + 1`
    ///
    /// Address wrapping is handled automatically within the 64 KiB space.
    pub fn mem_read16_at(&self, addr: u16) -> u16 {
        let a = addr as usize;
        let lo = self.mem[a % 0x10000] as u16;
        let hi = self.mem[(a.wrapping_add(1)) % 0x10000] as u16;
        (hi << 8) | lo
    }

    /// Writes a single byte to the VRAM data port.
    ///
    /// The first call stores the byte internally as the low byte of a pending 16-bit word.
    /// The second call provides the high byte, at which point the full word is written to memory
    /// and the address is automatically advanced.
    ///
    /// After a completed write, any pending read buffer state is cleared.
    pub fn write_data_port(&mut self, value: u8) {
        if self.addr_latch_low.is_none() {
            self.addr_latch_low = Some(value);
        } else {
            let low = self.addr_latch_low.take().unwrap();
            let word = (value as u16) << 8 | (low as u16);
            let addr = (self.vma_addr & 0xFFFF) as u16;
            self.mem_write16_at(addr, word);
            self.vma_addr = (self.vma_addr.wrapping_add(self.auto_inc as u32)) & 0xFFFF;
            self.read_buffer_valid = false;
            self.read_byte_index = 0;
        }
    }

    /// Reads a single byte from the VRAM data port.
    ///
    /// Reads are buffered internally. When the buffer is not valid, the function returns the previously buffered value
    /// (or zero initially) and immediately refills the buffer from the current address.
    /// Subsequent reads return the low byte first, then the high byte of the buffered word.
    ///
    /// After the high byte is returned, the buffer is refreshed from the next
    /// address and the address is automatically advanced.
    pub fn read_data_port_byte(&mut self) -> u8 {
        if !self.read_buffer_valid {
            // First read after address change: per hardware the returned value is the previous buffer (we assume zero)
            // But immediately fill the buffer from current VMA and set index so next read returns low byte.
            let returned = (self.read_buffer & 0xFF) as u8; // previous buffered low (initially 0)
            // refill buffer from current VMA
            let addr = (self.vma_addr & 0xFFFF) as u16;
            self.read_buffer = self.mem_read16_at(addr);
            self.read_buffer_valid = true;
            self.read_byte_index = 0; // next read returns low byte from just-filled buffer
            // On some hardware semantics the buffer refill may increment VMA immediately (we follow that)
            self.vma_addr = (self.vma_addr.wrapping_add(self.auto_inc as u32)) & 0xFFFF;
            return returned;
        } else {
            // Buffer valid: return byte depending on index
            let out = if self.read_byte_index == 0 {
                (self.read_buffer & 0xFF) as u8
            } else {
                (self.read_buffer >> 8) as u8
            };

            // advance index
            self.read_byte_index = (self.read_byte_index + 1) & 1;

            // If we've just returned the high byte (index wrapped to 0), then refill buffer from current VMA and increment VMA.
            if self.read_byte_index == 0 {
                let addr = (self.vma_addr & 0xFFFF) as u16;
                self.read_buffer = self.mem_read16_at(addr);
                self.vma_addr = (self.vma_addr.wrapping_add(self.auto_inc as u32)) & 0xFFFF;
            }
            return out;
        }
    }

    /// Helper: read a full 16-bit word via data port sequence (two calls).
    ///
    /// This method performs two consecutive byte reads and combines them into a single 16-bit value,
    /// returning the low byte first and the high byte second.
    pub fn read_word_via_port(&mut self) -> u16 {
        let lo = self.read_data_port_byte() as u16;
        let hi = self.read_data_port_byte() as u16;
        (hi << 8) | lo
    }
}

#[cfg(test)]
mod vram_tests {
    use super::Vram;

    /// Verifies the power-on state of VRAM.
    ///
    /// Ensures memory is zero-initialized and the address starts at 0x0000.
    #[test]
    fn power_on_state() {
        let v = Vram::new();

        assert_eq!(v.get_addr(), 0x0000);
        assert_eq!(v.mem.len(), 0x10000);
    }

    /// Checks that load_from_slice correctly copies data into VRAM memory.
    #[test]
    fn load_from_slice_copies_data() {
        let mut v = Vram::new();
        let data = [0xAA, 0xBB, 0xCC, 0xDD];

        v.load_from_slice(&data);

        assert_eq!(v.mem[0], 0xAA);
        assert_eq!(v.mem[1], 0xBB);
        assert_eq!(v.mem[2], 0xCC);
        assert_eq!(v.mem[3], 0xDD);
    }

    /// Ensures that 16-bit direct memory writes and reads work correctly.
    #[test]
    fn mem_write_and_read_16bit() {
        let mut v = Vram::new();

        v.mem_write16_at(0x2000, 0xABCD);
        let value = v.mem_read16_at(0x2000);

        assert_eq!(value, 0xABCD);
        assert_eq!(v.mem[0x2000], 0xCD); // low byte
        assert_eq!(v.mem[0x2001], 0xAB); // high byte
    }

    /// Verifies that low and high address writes correctly form a 16-bit address.
    #[test]
    fn address_low_high_writes() {
        let mut v = Vram::new();

        v.write_addr_low(0x34);
        v.write_addr_high(0x12);

        assert_eq!(v.get_addr(), 0x1234);
    }

    /// Ensures that changing the address resets the internal read buffer state.
    #[test]
    fn set_addr_resets_buffer_state() {
        let mut v = Vram::new();

        v.mem_write16_at(0x4000, 0x1111);
        v.set_addr(0x4000);

        let first = v.read_data_port_byte();
        assert_eq!(first, 0x00); // buffered delay
    }

    /// Verifies that the automatic address increment is applied after a write.
    #[test]
    fn auto_increment_is_applied_after_write() {
        let mut v = Vram::new();
        v.set_addr(0x3000);
        v.set_auto_increment(2);

        v.write_data_port(0xAA);
        v.write_data_port(0xBB);

        assert_eq!(v.get_addr(), 0x3002);
    }

    /// Ensures that writing through the data port produces the correct 16-bit word.
    #[test]
    fn write_data_port_writes_correct_word() {
        let mut v = Vram::new();
        v.set_addr(0x5000);

        v.write_data_port(0x5A);
        v.write_data_port(0xA5);

        let value = v.mem_read16_at(0x5000);
        assert_eq!(value, 0xA55A);
    }

    /// Verifies the buffered read behavior of the VRAM data port.
    ///
    /// The first read returns a delayed value, followed by low and high bytes.
    #[test]
    fn buffered_read_behavior() {
        let mut v = Vram::new();
        v.mem_write16_at(0x6000, 0xBEEF);
        v.set_addr(0x6000);

        let first = v.read_data_port_byte();
        let lo = v.read_data_port_byte();
        let hi = v.read_data_port_byte();

        assert_eq!(first, 0x00); // delayed buffer read
        assert_eq!(lo, 0xEF);
        assert_eq!(hi, 0xBE);
    }

    /// Ensures that read_word_via_port returns a full 16-bit value correctly.
    #[test]
    fn read_word_via_port_reads_correct_value() {
        let mut v = Vram::new();
        v.mem_write16_at(0x7000, 0xCAFE);
        v.set_addr(0x7000);

        let _ = v.read_data_port_byte(); // discard buffered read
        let value = v.read_word_via_port();

        assert_eq!(value, 0xCAFE);
    }

    /// Verifies correct address wrapping at the 64 KiB boundary.
    #[test]
    fn address_wraps_around_64k() {
        let mut v = Vram::new();
        v.set_addr(0xFFFF);

        v.write_data_port(0x11);
        v.write_data_port(0x22);

        assert_eq!(v.get_addr(), 0x0000);
        assert_eq!(v.mem_read16_at(0xFFFF), 0x2211);
    }
}

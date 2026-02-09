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
    vma_addr: u16,
    vmain: u8, // $2115
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
            vmain: 0,
            addr_latch_low: None,
            read_buffer: 0,
            read_buffer_valid: false,
            read_byte_index: 0,
            auto_inc: 1,
        }
    }

    /// Writes a value to the VMAIN register ($2115).
    ///
    /// This register controls the VRAM address increment mode:
    /// - bits 0-1: increment value selection (1, 32, 128)
    /// - bit 7: increment on high/low byte
    pub fn write_vmain(&mut self, value: u8) {
        self.vmain = value;
    }

    /// Returns the increment value based on the current VMAIN setting.
    ///
    /// Determines how much the VRAM address should advance after a read or write
    /// depending on bits 0-1 of VMAIN.
    fn increment_value(&self) -> u16 {
        match self.vmain & 0b11 {
            0 => 1,
            1 => 32,
            2 => 128,
            _ => 1,
        }
    }

    /// Checks if the increment should occur after reading/writing the high byte.
    ///
    /// Controlled by bit 7 of VMAIN. Returns true if increment happens on high byte.
    fn increment_on_high(&self) -> bool {
        self.vmain & 0x80 != 0
    }

    /// Increments the VRAM address if the current byte (high or low) matches the VMAIN increment setting.
    ///
    /// `is_high` indicates whether the current operation is on the high byte.
    /// Only increments if the VMAIN setting matches the byte being processed.
    fn maybe_increment(&mut self, is_high: bool) {
        if self.increment_on_high() == is_high {
            self.vma_addr = self.vma_addr.wrapping_add(self.increment_value());
        }
    }

    /// Writes the low byte of the VRAM address ($2116).
    ///
    /// This sets only the lower 8 bits of `vma_addr`.
    /// Any buffered read state is cleared to start a new read sequence.
    pub fn write_addr_low(&mut self, value: u8) {
        self.vma_addr = (self.vma_addr & 0xFF00) | value as u16;
        self.read_buffer_valid = false;
    }

    /// Writes the high byte of the VRAM address ($2117).
    ///
    /// This sets only the upper 8 bits of `vma_addr`.
    /// Any buffered read state is cleared to start a new read sequence.
    pub fn write_addr_high(&mut self, value: u8) {
        self.vma_addr = ((value as u16) << 8) | (self.vma_addr & 0x00FF);
        self.read_buffer_valid = false;
    }

    /// Computes the effective memory address for the current VRAM address.
    ///
    /// This is mainly used internally to index the `mem` vector.
    /// The address is doubled (shifted left 1) because VRAM stores 16-bit words in little-endian.
    fn mem_addr(&self) -> usize {
        ((self.vma_addr as usize) << 1) & 0xFFFF
    }

    /// Reads a 16-bit word from the current VRAM address.
    ///
    /// Returns the word in little-endian order: low byte first, high byte second.
    fn read_word(&self) -> u16 {
        self.mem_read16_at(self.vma_addr)
    }

    /// Writes a 16-bit word to the current VRAM address.
    ///
    /// The word is stored in little-endian: low byte at `addr`, high byte at `addr + 1`.
    fn write_word(&mut self, word: u16) {
        self.mem_write16_at(self.vma_addr, word);
    }

    /// Writes the low byte of a 16-bit word to the VRAM data port ($2118).
    ///
    /// The value is temporarily stored in `addr_latch_low`.
    /// Any buffered read state is invalidated. Address increment may occur based on VMAIN.
    pub fn write_data_low(&mut self, value: u8) {
        self.addr_latch_low = Some(value);
        self.maybe_increment(false);
        self.read_buffer_valid = false;
    }

    /// Writes the high byte of a 16-bit word to the VRAM data port ($2119).
    ///
    /// Combines the previously latched low byte with this high byte to form a full 16-bit word,
    /// which is then written to VRAM. Address increment may occur based on VMAIN.
    pub fn write_data_high(&mut self, value: u8) {
        let low = self.addr_latch_low.take().unwrap_or(0);
        let word = (value as u16) << 8 | low as u16;
        self.write_word(word);
        self.maybe_increment(true);
        self.read_buffer_valid = false;
    }

    //  Reads $2139 / $213A/// Reads the low byte from VRAM via the data port ($2139).
    ///
    /// Uses the internal read buffer. If the buffer is invalid, it is loaded from VRAM.
    /// Address increment may occur depending on VMAIN settings.
    pub fn read_data_low(&mut self) -> u8 {
        if !self.read_buffer_valid {
            self.read_buffer = self.read_word();
            self.read_buffer_valid = true;
        }
        let out = (self.read_buffer & 0xFF) as u8;
        self.maybe_increment(false);
        out
    }

    /// Reads the high byte from VRAM via the data port ($213A).
    ///
    /// Uses the internal read buffer. If the buffer is invalid, it is loaded from VRAM.
    /// Address increment may occur depending on VMAIN settings.
    pub fn read_data_high(&mut self) -> u8 {
        if !self.read_buffer_valid {
            self.read_buffer = self.read_word();
            self.read_buffer_valid = true;
        }
        let out = (self.read_buffer >> 8) as u8;
        self.maybe_increment(true);
        out
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

    /// Directly sets the current VRAM address using a full 16-bit value.
    ///
    /// This bypasses the low/high write sequence and replaces the address entirely.
    /// Any buffered read state is cleared so that subsequent reads behave
    /// as if the address has just been changed.
    pub fn set_addr(&mut self, addr: u16) {
        self.vma_addr = addr;
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
    pub fn mem_write16_at(&mut self, addr_word: u16, word: u16) {
        let byte_addr = (addr_word as usize) << 1;
        
        self.mem[byte_addr & 0xFFFF] = (word & 0xFF) as u8;
        self.mem[(byte_addr + 1) & 0xFFFF] = (word >> 8) as u8;
    }

    /// Reads a 16-bit word directly from VRAM memory at the given address.
    ///
    /// The value is reconstructed assuming little-endian layout:
    /// - low byte from `addr`
    /// - high byte from `addr + 1`
    ///
    /// Address wrapping is handled automatically within the 64 KiB space.
    pub fn mem_read16_at(&self, addr_word: u16) -> u16 {
        let byte_addr = (addr_word as usize) << 1;
        let lo = self.mem[byte_addr & 0xFFFF] as u16;
        let hi = self.mem[(byte_addr + 1) & 0xFFFF] as u16;
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

            let addr = self.vma_addr;
            self.mem_write16_at(addr, word);

            self.vma_addr = self.vma_addr.wrapping_add(self.auto_inc);

            self.read_buffer_valid = false;
            self.read_byte_index = 0;
        }
    }

    /// Reads the low byte of the current VRAM data port word.
    ///
    /// If the internal read buffer is not valid, it is first filled from the current
    /// VRAM address. The low byte of the buffered word is then returned.
    /// The buffer remains valid, so a subsequent call to read the high byte will return the upper byte of the same word.
    pub fn read_data_port_low(&mut self) -> u8 {
        if !self.read_buffer_valid {
            let addr = self.vma_addr;
            self.read_buffer = self.mem_read16_at(addr);
            self.read_buffer_valid = true;
        }

        (self.read_buffer & 0x00FF) as u8
    }

    /// Reads the high byte of the current VRAM data port word.
    ///
    /// If the internal read buffer is not valid, it is first filled from the current
    /// VRAM address. The high byte of the buffered word is then returned.
    /// After returning the high byte, the VRAM address is automatically incremented
    /// by the configured auto-increment value, and the buffer is invalidated.
    pub fn read_data_port_high(&mut self) -> u8 {
        if !self.read_buffer_valid {
            let addr = self.vma_addr;
            self.read_buffer = self.mem_read16_at(addr);
            self.read_buffer_valid = true;
        }

        let hi = (self.read_buffer >> 8) as u8;

        // Increment VRAM address after reading high byte
        self.vma_addr = self.vma_addr.wrapping_add(self.auto_inc);
        self.read_buffer_valid = false;

        hi
    }

    /// Helper: read a full 16-bit word via data port sequence (two calls).
    ///
    /// This method performs two consecutive byte reads and combines them into a single 16-bit value,
    /// returning the low byte first and the high byte second.
    pub fn read_word_via_port(&mut self) -> u16 {
        let lo = self.read_data_port_low() as u16;
        let hi = self.read_data_port_high() as u16;
        (hi << 8) | lo
    }
}

#[cfg(test)]
mod vram_tests {
    use super::Vram;

    /// Verifies the power-on state of VRAM.
    ///
    /// Ensures memory is zero-initialized, the address starts at 0x0000,
    /// and internal buffers are cleared.
    #[test]
    fn power_on_state() {
        let v = Vram::new();

        assert_eq!(v.get_addr(), 0x0000);
        assert_eq!(v.mem.len(), 0x10000);
        assert!(v.mem.iter().all(|&b| b == 0));
    }

    /// Verifies that writing low and high address registers correctly
    /// forms a full 16-bit VRAM address.
    #[test]
    fn write_addr_low_high_sets_address() {
        let mut v = Vram::new();

        v.write_addr_low(0x34);
        v.write_addr_high(0x12);

        assert_eq!(v.get_addr(), 0x1234);
    }

    /// Ensures that changing the VRAM address invalidates the read buffer.
    /// The first read after an address change is a dummy read (value undefined).
    #[test]
    fn set_addr_clears_read_buffer() {
        let mut v = Vram::new();

        v.mem_write16_at(0x2000, 0xABCD);
        v.set_addr(0x2000);

        // First read is dummy
        let _dummy = v.read_data_port_low();

        // Now we can read the actual low and high bytes
        let lo = v.read_data_port_low();
        let hi = v.read_data_port_high();

        // Expected little-endian values from VRAM
        assert_eq!(lo, 0xCD);
        assert_eq!(hi, 0xAB);

        // VRAM address should NOT have incremented yet (dummy read + high byte read increments address in SNES)
        assert_eq!(v.get_addr(), 0x2001);
    }

    /// Ensures that the internal read buffer is reused between low and high reads.
    /// Requires an initial dummy read, as on real SNES hardware.
    #[test]
    fn read_buffer_is_shared_between_low_and_high() {
        let mut v = Vram::new();

        v.mem_write16_at(0x5000, 0x1234);
        v.set_addr(0x5000);

        // Dummy read
        let _ = v.read_data_port_low();

        // Next reads should return low/high of the same buffered word
        let lo = v.read_data_port_low();
        let hi = v.read_data_port_high();

        assert_eq!(lo, 0x34);
        assert_eq!(hi, 0x12);

        // After reading high byte, address increments
        assert_eq!(v.get_addr(), 0x5001);
    }

    /// Verifies that read_word_via_port correctly returns a full 16-bit word.
    /// Accounts for the initial dummy read after setting the VRAM address.
    #[test]
    fn read_word_via_port_reads_correct_value() {
        let mut v = Vram::new();

        v.mem_write16_at(0x6000, 0xDEAD);
        v.set_addr(0x6000);

        // Dummy read
        let _ = v.read_data_port_low();

        // read_word_via_port does low + high
        let value = v.read_word_via_port();
        assert_eq!(value, 0xDEAD);

        // After reading high byte of the word, VRAM address increments by 1 (default auto_inc)
        assert_eq!(v.get_addr(), 0x6001);
    }

    /// Ensures that load_from_slice copies data into VRAM starting at address 0.
    #[test]
    fn load_from_slice_copies_data() {
        let mut v = Vram::new();

        let data = [0x11, 0x22, 0x33, 0x44];
        v.load_from_slice(&data);

        assert_eq!(v.mem[0], 0x11);
        assert_eq!(v.mem[1], 0x22);
        assert_eq!(v.mem[2], 0x33);
        assert_eq!(v.mem[3], 0x44);
    }

    /// Verifies that the VRAM address correctly wraps around the 64 KiB boundary
    /// when writing through the VRAM data port (word-based addressing).
    #[test]
    fn address_wraps_around_64k() {
        let mut v = Vram::new();

        v.set_addr(0xFFFF);
        v.set_auto_increment(1);

        v.write_data_port(0xAA);
        v.write_data_port(0xBB);

        // Address wraps on word index
        assert_eq!(v.get_addr(), 0x0000);

        // VRAM uses word addressing => byte address = addr << 1
        let byte_addr = (0xFFFFu32 << 1) & 0xFFFF;
        let lo = v.mem[byte_addr as usize];
        let hi = v.mem[((byte_addr + 1) & 0xFFFF) as usize];

        assert_eq!((hi as u16) << 8 | lo as u16, 0xBBAA);
    }

    /// Ensures that VMAIN increment value is applied correctly (increment = 32).
    #[test]
    fn vmain_increment_32() {
        let mut v = Vram::new();

        // VMAIN bits 0–1 = 01 → increment 32
        v.write_vmain(0b0000_0001);
        v.set_addr(0x0100);

        v.write_data_low(0x00);
        v.write_data_high(0x11);

        assert_eq!(v.get_addr(), 0x0120);
    }

    /// Ensures that VMAIN increment-on-high behavior works correctly.
    #[test]
    fn vmain_increment_on_high_only() {
        let mut v = Vram::new();

        // bit 7 set → increment on high byte only
        v.write_vmain(0b1000_0000);
        v.set_addr(0x2000);

        v.write_data_low(0xAA);
        assert_eq!(v.get_addr(), 0x2000);

        v.write_data_high(0xBB);
        assert_eq!(v.get_addr(), 0x2001);
    }
}

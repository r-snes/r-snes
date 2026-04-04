/// Extension trait for [`u16`] which defines additional methods
/// allowing to "split" references (mutable or not) to a [`u16`] into
/// two separate references to its individual bytes (as [`u8`]s)
pub trait U16Split {
    /// Split an immutable reference to a [`u16`] into two
    /// immutable references to its bytes (as [`u8`]s)
    fn split<'a>(&'a self) -> SplitU16<'a>;
    /// Split a mutable reference to a [`u16`] into two
    /// mutable references to its bytes (as [`u8`]s)
    fn split_mut<'a>(&'a mut self) -> SplitU16Mut<'a>;

    /// Get an immutable reference to the least significant byte of a [`u16`]
    fn lo<'a>(&'a self) -> &'a u8 {
        self.split().lo
    }
    /// Get an immutable reference to the most significant byte of a [`u16`]
    fn hi<'a>(&'a self) -> &'a u8 {
        self.split().hi
    }

    /// Get an mutable reference to the least significant byte of a [`u16`]
    fn lo_mut<'a>(&'a mut self) -> &'a mut u8 {
        self.split_mut().lo
    }
    /// Get an mutable reference to the most significant byte of a [`u16`]
    fn hi_mut<'a>(&'a mut self) -> &'a mut u8 {
        self.split_mut().hi
    }
}

/// Helper struct which contains two immutable references
/// to the individual bytes of a [`u16`]
pub struct SplitU16<'a> {
    /// Ref to the least signicant byte
    pub lo: &'a u8,
    /// Ref to the most significant byte
    pub hi: &'a u8,
}

/// Helper struct which contains two immutable references
/// to the individual bytes of a [`u16`]
pub struct SplitU16Mut<'a> {
    /// Mut ref to the least significant byte
    pub lo: &'a mut u8,
    /// Mut ref to the most significant byte
    pub hi: &'a mut u8,
}

impl U16Split for u16 {
    fn split<'a>(&'a self) -> SplitU16<'a> {
        let first_byte_ptr = self as *const u16 as *const u8;
        let second_byte_ptr = unsafe { first_byte_ptr.add(1) };
        let (lo, hi) = if cfg!(target_endian = "little") {
            unsafe { (&*first_byte_ptr, &*second_byte_ptr) }
        } else {
            unsafe { (&*second_byte_ptr, &*first_byte_ptr) }
        };
        SplitU16 { lo, hi }
    }

    fn split_mut<'a>(&'a mut self) -> SplitU16Mut<'a> {
        let first_byte_ptr = self as *mut u16 as *mut u8;
        let second_byte_ptr = unsafe { first_byte_ptr.add(1) };
        let (lo, hi) = if cfg!(target_endian = "little") {
            unsafe { (&mut *first_byte_ptr, &mut *second_byte_ptr) }
        } else {
            unsafe { (&mut *second_byte_ptr, &mut *first_byte_ptr) }
        };
        SplitU16Mut { lo, hi }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_split_u16() {
        let my_u16: u16 = 0x1234;

        let byte_refs = my_u16.split();

        assert_eq!(*byte_refs.hi, 0x12);
        assert_eq!(*byte_refs.lo, 0x34);

        assert_eq!(my_u16, 0x1234);
    }

    #[test]
    fn get_individual_bytes() {
        let my_u16: u16 = 0xbeef;

        assert_eq!(*my_u16.hi(), 0xbe);
        assert_eq!(*my_u16.lo(), 0xef);
    }

    #[test]
    fn test_split_u16_mut() {
        let mut my_u16: u16 = 0x1234;

        let byte_refs = my_u16.split_mut();

        assert_eq!(*byte_refs.hi, 0x12);
        assert_eq!(*byte_refs.lo, 0x34);

        *byte_refs.lo = 0xee;

        // drop(byte_refs) // byte_refs is implicitly dropped as we start
        //                    reading from my_u16 again
        assert_eq!(my_u16, 0x12ee);

        // we can also assign directly from the method call
        *my_u16.hi_mut() = 0xaa;
        assert_eq!(my_u16, 0xaaee);

        *my_u16.lo_mut() = 0x42;
        assert_eq!(my_u16, 0xaa42);
    }
}

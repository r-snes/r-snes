pub trait Bits {
    const BITS: usize;
}

macro_rules! impl_bits {
    ($ty:ty, $n:expr) => {
        impl Bits for $ty {
            const BITS: usize = $n;
        }
    };
}

impl_bits!(u8, 8);
impl_bits!(u16, 16);
impl_bits!(u32, 32);
impl_bits!(u64, 64);
impl_bits!(u128, 128);
impl_bits!(bool, 1);

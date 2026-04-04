use std::ops::{
    Add,
    AddAssign,
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    BitXor,
    BitXorAssign,
    Shl,
    ShlAssign,
    Shr,
    ShrAssign,
    Sub,
    SubAssign,
    Not,
};
use std::cmp::Eq;
use duplicate::duplicate;

/// Trait describing values which the CPU operates on: u8 and u16
///
/// This enables us to use generics instead of code duplication for
/// code that should work for both sizes
pub(crate) trait Reg : Copy
    + Add<Output = Self>
    + AddAssign
    + BitAnd<Output = Self>
    + BitAndAssign
    + BitOr<Output = Self>
    + BitOrAssign
    + BitXor<Output = Self>
    + BitXorAssign
    + Shl<Output = Self>
    + ShlAssign
    + Shr<Output = Self>
    + ShrAssign
    + Sub<Output = Self>
    + SubAssign
    + Not<Output = Self>
    + Eq
{
    const ZERO: Self;
    const ONE: Self;
    const BITS: Self;

    /// Method of u8 and u16
    fn wrapping_add(self, other: Self) -> Self;

    /// Method of u8 and u16
    fn wrapping_sub(self, other: Self) -> Self;

    /// Method of u8 and u16
    fn carrying_add(self, other: Self, carry_in: bool) -> (Self, bool);

    /// Method of u8 and u16
    fn overflowing_sub(self, other: Self) -> (Self, bool);

    /// Checks for zero-equality (intended to be used for setting the Z flag for example)
    fn is_zero(self) -> bool {
        self == Self::ZERO
    }

    /// Checks for negative (intended to be used for setting the N flag)
    fn is_neg(self) -> bool {
        self & (Self::ONE << (Self::BITS - Self::ONE)) != Self::ZERO
    }
}

duplicate! {
    [
        DUP_type;
        [u8];
        [u16];
    ]
    impl Reg for DUP_type {
        const ZERO: Self = 0;
        const ONE: Self = 1;
        const BITS: Self = DUP_type::BITS as DUP_type;

        fn wrapping_add(self, other: Self) -> Self {
            self.wrapping_add(other)
        }

        fn wrapping_sub(self, other: Self) -> Self {
            self.wrapping_sub(other)
        }

        fn carrying_add(self, other: Self, carry_in: bool) -> (Self, bool) {
            self.carrying_add(other, carry_in)
        }

        fn overflowing_sub(self, other: Self) -> (Self, bool) {
            self.overflowing_sub(other)
        }
    }
}

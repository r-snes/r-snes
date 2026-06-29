use std::num::Wrapping;
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
    + AddBcd
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

pub(crate) trait AddBcd: Sized {
    fn add_bcd(self, other: Self, carry_in: bool) -> (Self, bool, bool);
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

impl AddBcd for u8 {
    fn add_bcd(self, other: Self, carry_in: bool) -> (Self, bool, bool) {
        use std::num::Wrapping as W;
        let op = W(other);
        let a = W(self);

        let mut ret: Wrapping<Self>;
        let mut c: bool = carry_in;

        ret = (a & W(0x0f)) + (op & W(0x0f)) + (W(c as u8) << 0);
        c = ret >= W(0xA); // new base 10 carry
        if c {
            // adjust the hex representation so that the hex digits
            // match the decimal representation of the value
            ret += 0x06;
        }
        ret = (a & W(0xf0)) + (op & W(0xf0)) + (W(c as u8) << 4) + (ret & W(0x0f));
        c = ret >= W(0xA0);

        let v = ((a ^ ret) & (op ^ ret)).0.is_neg();
        if c {
            ret += 0x60;
        }

        (ret.0, c, v)
    }
}

impl AddBcd for u16 {
    fn add_bcd(self, other: Self, carry_in: bool) -> (Self, bool, bool) {
        use std::num::Wrapping as W;
        let op = W(other);
        let a = W(self);

        let mut ret: Wrapping<Self>;
        let mut c: bool = carry_in;

        ret = (a & W(0x000f)) + (op & W(0x000f)) + (W(c as u16) << 0);
        c = ret >= W(0xA); // new base 10 carry
        if c {
            // adjust the hex representation so that the hex digits
            // match the decimal representation of the value
            ret += 0x0006;
        }

        ret = (a & W(0x00f0)) + (op & W(0x00f0)) + (W(c as u16) << 4) + (ret & W(0x000f));
        c = ret >= W(0xA0);
        if c {
            ret += 0x0060;
        }

        ret = (a & W(0x0f00)) + (op & W(0x0f00)) + (W(c as u16) << 8) + (ret & W(0x00ff));
        c = ret >= W(0xA00);
        if c {
            ret += 0x0600;
        }

        ret = (a & W(0xf000)) + (op & W(0xf000)) + (W(c as u16) << 12) + (ret & W(0x0fff));
        c = ret >= W(0xA000);

        let v = ((a ^ ret) & (op ^ ret)).0.is_neg();
        if c {
            ret += 0x6000;
        }

        (ret.0, c, v)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn simple_bcd16() {
        let (res, c_out, overflow) = 0x3550_u16.add_bcd(0x4470, false);

        assert_eq!(res, 0x8020, "res was {res:#.4X} instead 0x8020");
        assert!(!c_out);
        assert!(overflow);
    }
}

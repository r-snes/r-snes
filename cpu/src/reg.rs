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
    + SubBcd
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

pub(crate) trait SubBcd: Sized {
    fn sub_bcd(self, other: Self, carry_in: bool) -> (Self, bool, bool);
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

        ret = (a & W(0x0f)) + (op & W(0x0f)) + W(c as u8);
        c = ret >= W(0xA); // new base 10 carry
        if c {
            // adjust the hex representation so that the hex digits
            // match the decimal representation of the value
            ret += 0x06;
        }

        ret += (a & W(0xf0)) + (op & W(0xf0));
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

        ret = (a & W(0x000f)) + (op & W(0x000f)) + W(c as u16);
        c = ret >= W(0xA); // new base 10 carry
        if c {
            // adjust the hex representation so that the hex digits
            // match the decimal representation of the value
            ret += 0x0006;
        }

        ret += (a & W(0x00f0)) + (op & W(0x00f0));
        c = ret >= W(0xA0);
        if c {
            ret += 0x0060;
        }

        ret += (a & W(0x0f00)) + (op & W(0x0f00));
        c = ret >= W(0xA00);
        if c {
            ret += 0x0600;
        }

        ret += (a & W(0xf000)) + (op & W(0xf000));
        c = ret >= W(0xA000);

        let v = ((a ^ ret) & (op ^ ret)).0.is_neg();
        if c {
            ret += 0x6000;
        }

        (ret.0, c, v)
    }
}

impl SubBcd for u8 {
    fn sub_bcd(self, other: Self, carry_in: bool) -> (Self, bool, bool) {
        use std::num::Wrapping as W;
        let op = W(other) + W(1);
        let a = W(self) + W(carry_in as u8);

        let mut ret = a;

        if ret & W(0xF) < op & W(0xF) {
            ret -= 0x6;
        }
        ret -= op & W(0xF);

        let c = !(ret & W(0xF0) < op & W(0xF0));
        let v = (!(self ^ !other) & (self ^ ret.0)).is_neg();
        if !c {
            ret -= 0x60;
        }
        ret -= op & W(0xF0);

        (ret.0, c, v)
    }
}

impl SubBcd for u16 {
    fn sub_bcd(self, other: Self, carry_in: bool) -> (Self, bool, bool) {
        use std::num::Wrapping as W;
        let op = W(other) + W(1);
        let a = W(self) + W(carry_in as u16);

        let mut ret = a;

        if ret & W(0xF) < op & W(0xF) {
            ret -= 0x6;
        }
        ret -= op & W(0xF);

        if ret & W(0xF0) < op & W(0xF0) {
            ret -= 0x60;
        }
        ret -= op & W(0xF0);

        if ret & W(0xF00) < op & W(0xF00) {
            ret -= 0x600;
        }
        ret -= op & W(0xF00);

        let c = !(ret & W(0xF000) < op & W(0xF000));
        let v = ((self ^ ret.0) & (!other ^ ret.0)).is_neg();
        if !c {
            ret -= 0x6000;
        }
        ret -= op & W(0xF000);

        (ret.0, c, v)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn simple_add_bcd16() {
        let (res, c_out, overflow) = 0x3550_u16.add_bcd(0x4470, false);

        assert_eq!(res, 0x8020, "res was {res:#.4X} instead 0x8020");
        assert!(!c_out);
        assert!(overflow);
    }

    #[test]
    fn simple_add_bcd8() {
        let (res, c_out, overflow) = 0x9_u8.add_bcd(0x9, false);

        assert_eq!(res, 0x18, "res was {res:#.2X} instead 0x18");
        assert!(!c_out);
        assert!(!overflow);
    }

    #[test]
    fn simple_sub_bcd16() {
        let (res, c_out, overflow) = 0x2345_u16.sub_bcd(0x1111, true);

        assert_eq!(res, 0x1234, "res was {res:#.4X} instead of 0x1234");
    }

    #[test]
    fn borrowing_sub_bcd16() {
        let (res, c_out, overflow) = 0x2345_u16.sub_bcd(0x1346, true);

        assert_eq!(res, 0x0999, "res was {res:#.4X} instead of 0x0999");
    }

    #[test]
    fn result_zero_sub_bcd16() {
        let (res, c_out, overflow) = 0x9090_u16.sub_bcd(0x9089, false);

        assert_eq!(res, 0);
    }

    #[test]
    fn sub_bcd_zero_minus_one() {
        let (res, c_out, overflow) = 0_u16.sub_bcd(1, true);

        assert_eq!(res, 0x9999);
        assert!(!c_out);
        assert!(overflow);
    }
}

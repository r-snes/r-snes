#![cfg(test)]

use tiny_bitfield_proc_macro::bitfield_read;

#[test]
fn simple_4_4() {
    #[rustfmt::skip]
    let x: u8 =      0b11000011;
    bitfield_read!(x : aaaabbbb);

    assert_eq!(a, 0b1100);
    assert_eq!(b, 0b0011);
}

#[test]
fn read_16_bit() {
    #[rustfmt::skip]
    let x: u16 =     0b1110100001001011;
    bitfield_read!(x : Pvvvii___ccccczz);

    assert_eq!(P, 1);
    assert_eq!(v, 0b110);
    assert_eq!(i, 0b10);
    assert_eq!(c, 0b10010);
    assert_eq!(z, 0b11);
}

#[test]
fn renames() {
    bitfield_read!(0b11111010 : fffbbbbb (foo = f; bar = b;));

    assert_eq!(foo, 0b111);
    assert_eq!(bar, 0b11010);
}

#[test]
fn multiple_idents() {
    bitfield_read!(0b1110_0101_1100_0111 : aaaa bbbb bbbb cccc);

    assert_eq!(a, 0b1110);
    assert_eq!(b, 0b01011100);
    assert_eq!(c, 0b0111);
}

#[test]
fn retypes() {
    #[expect(clippy::unusual_byte_groupings)]
    {
        // widening retype; no rename
        bitfield_read!(0b111_001_10 : aaabbbcc (a: u16; b: u32));

        let _: u16 = a; // would fail to compile if `a` wasn't u16
        let _: u32 = b; // same
        let _: u8 = c; // same

        assert_eq!(a, 0b111);
        assert_eq!(b, 0b001);
        assert_eq!(c, 0b10);
    }

    {
        // shortening retype + rename
        bitfield_read!(0xAAFF : hhhhhhhh llllllll (hi: u8 = h; lo: u8 = l));

        assert_eq!(hi, 0xAA_u8);
        assert_eq!(lo, 0xFF_u8);
    }
}

#[test]
fn complex_decode() {
    let data = [0b1011_0110_u8];

    bitfield_read!(data[0] : tfFFbbbB (
        b1: bool = t;
        b2: bool = f;
        foo = F;
        bar = b;
        baz = B;
    ));
    assert!(b1);
    assert!(!b2);
    assert_eq!(foo, 0b11);
    assert_eq!(bar, 0b011);
    assert_eq!(baz, 0);
}

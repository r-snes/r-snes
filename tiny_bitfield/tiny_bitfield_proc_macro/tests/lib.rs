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

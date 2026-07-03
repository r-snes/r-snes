use crate::reg::Reg;
use crate::registers::RegisterP;

pub fn set_nz<T: Reg>(val: T, p: &mut RegisterP) {
    p.Z = val.is_zero();
    p.N = val.is_neg();
}

pub fn eor<T: Reg>(a: &mut T, idb: T, p: &mut RegisterP) {
    *a ^= idb;
    set_nz(*a, p);
}

pub fn and<T: Reg>(a: &mut T, idb: T, p: &mut RegisterP) {
    *a &= idb;
    set_nz(*a, p);
}

pub fn ora<T: Reg>(a: &mut T, idb: T, p: &mut RegisterP) {
    *a |= idb;
    set_nz(*a, p);
}

pub fn cmp<T: Reg>(a: &mut T, idb: T, p: &mut RegisterP) {
    let (diff, overflow) = a.overflowing_sub(idb);

    p.C = !overflow;
    set_nz(diff, p);
}

pub fn adc<T: Reg>(a: &mut T, idb: T, p: &mut RegisterP) {
    if p.D {
        todo!("decimal mode is not supported yet");
    } else {
        let (res, carry_out) = a.carrying_add(idb, p.C);

        p.C = carry_out;
        p.V = ((*a ^ res) & (idb ^ res)).is_neg();
        *a = res;
        set_nz(*a, p);
    }
}

pub fn sbc<T: Reg>(a: &mut T, idb: T, p: &mut RegisterP) {
    adc(a, !idb, p)
}

pub fn bit<T: Reg>(a: &mut T, idb: T, p: &mut RegisterP) {
    p.Z = *a & idb == T::ZERO;
    p.N = idb.is_neg();
    p.V = (idb << T::ONE).is_neg();
}

// BIT immediate only sets Z, doesn't touch N and V
pub fn bit_imm<T: Reg>(a: &mut T, idb: T, p: &mut RegisterP) {
    p.Z = *a & idb == T::ZERO;
}

pub fn asl<T: Reg>(op: &mut T, p: &mut RegisterP) {
    p.C = op.is_neg();

    *op <<= T::ONE;
    set_nz(*op, p);
}

pub fn lsr<T: Reg>(op: &mut T, p: &mut RegisterP) {
    p.C = *op & T::ONE != T::ZERO;

    *op >>= T::ONE;
    set_nz(*op, p);
}

pub fn rol<T: Reg>(op: &mut T, p: &mut RegisterP) {
    let carry_in = if p.C { T::ONE } else { T::ZERO };
    let carry_out = op.is_neg();

    *op = (*op << T::ONE) | carry_in;
    p.C = carry_out;
    set_nz(*op, p);
}

pub fn ror<T: Reg>(op: &mut T, p: &mut RegisterP) {
    let carry_in = if p.C {
        T::ONE << (T::BITS - T::ONE)
    } else {
        T::ZERO
    };
    let carry_out = *op & T::ONE != T::ZERO;

    *op = (*op >> T::ONE) | carry_in;
    p.C = carry_out;
    set_nz(*op, p);
}

pub fn inc<T: Reg>(op: &mut T, p: &mut RegisterP) {
    *op = op.wrapping_add(T::ONE);
    set_nz(*op, p);
}

pub fn dec<T: Reg>(op: &mut T, p: &mut RegisterP) {
    *op = op.wrapping_sub(T::ONE);
    set_nz(*op, p);
}

pub fn tsb<T: Reg>(op: &mut T, a: T, p: &mut RegisterP) {
    p.Z = (*op & a) == T::ZERO;
    *op |= a;
}

pub fn trb<T: Reg>(op: &mut T, a: T, p: &mut RegisterP) {
    p.Z = (*op & a) == T::ZERO;
    *op &= !a;
}

#[cfg(test)]
mod tests {
    use crate::registers::RegisterP;
    use duplicate::duplicate_item;

    #[test]
    fn eor() {
        let mut p = RegisterP::default();
        let mut acc: u8 = 0b11110001;
        let operand: u8 = 0b00011111;
        let expectd: u8 = 0b11101110;

        super::eor(&mut acc, operand, &mut p);
        assert_eq!(acc, expectd);
        assert!(!p.Z);
        assert!(p.N);
    }

    #[test]
    fn and() {
        let mut p = RegisterP::default();
        let mut acc: u8 = 0b11110001;
        let operand: u8 = 0b00011111;
        let expectd: u8 = 0b00010001;

        super::and(&mut acc, operand, &mut p);
        assert_eq!(acc, expectd);
        assert!(!p.Z);
        assert!(!p.N);
    }

    #[test]
    fn ora() {
        let mut p = RegisterP::default();
        let mut acc: u8 = 0b11110001;
        let operand: u8 = 0b00011111;
        let expectd: u8 = 0b11111111;

        super::ora(&mut acc, operand, &mut p);
        assert_eq!(acc, expectd);
        assert!(!p.Z);
        assert!(p.N);
    }

    // duplicate over multiple cases of CMP
    // lt: lesser than; gt: greater_than
    // pp: both operands positive; pn positve - negative; nn: neg-neg...
    // flag Z is the zero-ness of the difference of the operands: only true when equal
    // flag N is the sign of the difference of the signed operands (1 when neg)
    // flag C is the sign of the difference of the unsigned operands
    //
    // C and N are equal when operand signs differ
    #[duplicate_item(
        DUP_name    DUP_a   DUP_op  DUP_z   DUP_n   DUP_c;
        [cmp_lt_pp] [0x11]  [0x22]  [false] [true]  [false];
        [cmp_lt_nn] [0x81]  [0x82]  [false] [true]  [false];
        [cmp_lt_np] [0xf0]  [0x22]  [false] [true]  [true];
        [cmp_gt_pn] [0x03]  [0x83]  [false] [true]  [false];
        [cmp_gt_pp] [0x22]  [0x11]  [false] [false] [true];
        [cmp_gt_nn] [0xbb]  [0xaa]  [false] [false] [true];
        [cmp_eq]    [0xff]  [0xff]  [true]  [false] [true];
    )]
    #[test]
    fn DUP_name() {
        let mut p = RegisterP::default();
        let mut acc: u8 = DUP_a;
        let operand: u8 = DUP_op;

        super::cmp(&mut acc, operand, &mut p);
        assert_eq!(acc, DUP_a, "A shouldn't change value");
        assert_eq!(p.Z, DUP_z);
        assert_eq!(p.C, DUP_c);
    }

    #[test]
    fn adc_1_plus_1() {
        let mut p = RegisterP::default();
        let mut acc: u16 = 0x101;
        let operand: u16 = 0x101;

        super::adc(&mut acc, operand, &mut p);
        assert_eq!(acc, 0x202);
        assert!(!p.Z);
        assert!(!p.N);
        assert!(!p.C);
        assert!(!p.V);
    }

    // duplicate over all possible output flags for ADC
    #[duplicate_item(
        DUP_name    DUP_a   DUP_op  DUP_c_in    DUP_res     DUP_z   DUP_n   DUP_v   DUP_c_out;
        [adc_____]  [1]     [1]     [true]      [3]         [false] [false] [false] [false];
        [adc____c]  [0x0006][0xfffb][false]     [1]         [false] [false] [false] [true];
        //   __v_ impossible: overflow implies C or N
        [adc___vc]  [0x8000][0x8001][false]     [1]         [false] [false] [true]  [true];
        [adc__n__]  [0x9000][0x0003][false]     [0x9003]    [false] [true]  [false] [false];
        [adc__n_c]  [0xf000][0xf000][false]     [0xe000]    [false] [true]  [false] [true];
        [adc__nv_]  [0x4000][0x4000][false]     [0x8000]    [false] [true]  [true]  [false];
        //   _nvc impossible: can't both overflow and carry with a negative result
        [adc_z___]  [0]     [0]     [false]     [0]         [true]  [false] [false] [false];
        [adc_z__c]  [0x0005][0xfffb][false]     [0]         [true]  [false] [false] [true];
        //   z_v_ impossible: z_v implies c
        [adc_z_vc]  [0x8000][0x8000][false]     [0]         [true]  [false] [true]  [true];
        //   zn** impossible: 0 is positive
    )]
    #[allow(non_snake_case)]
    #[test]
    fn DUP_name() {
        let mut p = RegisterP::default();
        let mut acc: u16 = DUP_a;
        let operand: u16 = DUP_op;
        p.C = DUP_c_in;

        super::adc(&mut acc, operand, &mut p);
        assert_eq!(acc, DUP_res);
        assert_eq!(p.Z, DUP_z);
        assert_eq!(p.N, DUP_n);
        assert_eq!(p.C, DUP_c_out);
        assert_eq!(p.V, DUP_v);
    }

    // for sbc, we only test a basic case it calls adc internally anyways
    #[test]
    fn sbc() {
        let mut p = RegisterP::default();
        let mut acc: u16 = 5;
        let operand: u16 = 3;

        // sbc does (acc = acc - operand - 1 + carry), so we set the carry
        // so that we can have 2 = 5 - 3
        p.C = true;

        super::sbc(&mut acc, operand, &mut p);
        assert_eq!(acc, 2);
    }

    // duplicate over 6 possible output flag combinations of asl
    #[duplicate_item(
        DUP_name    DUP_a       DUP_res     DUP_c   DUP_n   DUP_z;
        [asl_]      [0b00000001][0b00000010][false] [false] [false];
        [asl_z]     [0b00000000][0b00000000][false] [false] [true];
        [asl_n]     [0b01000000][0b10000000][false] [true]  [false];
        //  _nz : impossible, 0 is positive
        [asl_c]     [0b10000001][0b00000010][true]  [false] [false];
        [asl_cz]    [0b10000000][0b00000000][true]  [false] [true];
        [asl_cn]    [0b11000000][0b10000000][true]  [true]  [false];
        //  _cnz : impossible, 0 is positive
    )]
    #[test]
    fn DUP_name() {
        let mut a: u8 = DUP_a;
        let mut p = RegisterP::default();

        p.C = !DUP_c;
        p.N = !DUP_n;
        p.Z = !DUP_z;

        super::asl(&mut a, &mut p);

        assert_eq!(a, DUP_res);
        assert_eq!(p.C, DUP_c);
        assert_eq!(p.N, DUP_n);
        assert_eq!(p.Z, DUP_z);
    }

    // duplicate over the possible output flag combinations of lsr
    #[duplicate_item(
        DUP_name    DUP_a       DUP_res     DUP_c   DUP_z;
        [lsr_]      [0b00000010][0b00000001][false] [false];
        [lsr_z]     [0b00000000][0b00000000][false] [true];
        [lsr_c]     [0b10000001][0b01000000][true]  [false];
        [lsr_cz]    [0b00000001][0b00000000][true]  [true];
    )]
    #[test]
    fn DUP_name() {
        let mut a: u8 = DUP_a;
        let mut p = RegisterP::default();

        super::lsr(&mut a, &mut p);

        assert_eq!(a, DUP_res);
        assert_eq!(p.C, DUP_c);
        assert_eq!(p.N, false, "N is always false as we shift right");
        assert_eq!(p.Z, DUP_z);
    }

    // duplicate over 6 possible output flag combinations of rol
    #[duplicate_item(
        DUP_name    DUP_a       DUP_c_in    DUP_res     DUP_c_out   DUP_n   DUP_z;
        [rol_]      [0b00000001][false]     [0b00000010][false]     [false] [false];
        [rolc_]     [0b00000001][true]      [0b00000011][false]     [false] [false];
        [rol_z]     [0b00000000][false]     [0b00000000][false]     [false] [true];
        [rol_n]     [0b01000000][false]     [0b10000000][false]     [true]  [false];
        [rolc_n]    [0b01000000][true]      [0b10000001][false]     [true]  [false];
        //  _nz : impossible, 0 is positive
        [rol_c]     [0b10000001][false]     [0b00000010][true]      [false] [false];
        [rolc_c]    [0b10000001][true]      [0b00000011][true]      [false] [false];
        [rol_cz]    [0b10000000][false]     [0b00000000][true]      [false] [true];
        [rol_cn]    [0b11000000][false]     [0b10000000][true]      [true]  [false];
        [rolc_cn]   [0b11000000][true]      [0b10000001][true]      [true]  [false];
        //  _cnz : impossible, 0 is positive
    )]
    #[test]
    fn DUP_name() {
        let mut a: u8 = DUP_a;
        let mut p = RegisterP::default();

        p.C = DUP_c_in;
        p.N = !DUP_n;
        p.Z = !DUP_z;

        let b: u8 = (1 << 1) | 1;
        println!("b: {b}");

        super::rol(&mut a, &mut p);

        assert_eq!(a, DUP_res);
        assert_eq!(p.C, DUP_c_out);
        assert_eq!(p.N, DUP_n);
        assert_eq!(p.Z, DUP_z);
    }

    // duplicate over 6 possible output flag combinations of ror
    #[duplicate_item(
        DUP_name    DUP_a       DUP_c_in    DUP_res     DUP_c_out   DUP_n   DUP_z;
        [ror_]      [0b00000010][false]     [0b00000001][false]     [false] [false];
        [ror_z]     [0b00000000][false]     [0b00000000][false]     [false] [true];
        [rorc_n]    [0b01000000][true]      [0b10100000][false]     [true]  [false];
        [ror_c]     [0b10000001][false]     [0b01000000][true]      [false] [false];
        [ror_cz]    [0b00000001][false]     [0b00000000][true]      [false] [true];
        [rorc_cn]   [0b11000001][true]      [0b11100000][true]      [true]  [false];
    )]
    #[test]
    fn DUP_name() {
        let mut a: u8 = DUP_a;
        let mut p = RegisterP::default();

        p.C = DUP_c_in;
        p.N = !DUP_n;
        p.Z = !DUP_z;

        let b: u8 = (1 << 1) | 1;
        println!("b: {b}");

        super::ror(&mut a, &mut p);

        assert_eq!(a, DUP_res);
        assert_eq!(p.C, DUP_c_out);
        assert_eq!(p.N, DUP_n);
        assert_eq!(p.Z, DUP_z);
    }

    #[test]
    fn inc() {
        let mut a: u8 = 126;
        let mut p = RegisterP::default();

        super::inc(&mut a, &mut p);
        assert_eq!(a, 127);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, false);

        super::inc(&mut a, &mut p);
        assert_eq!(a, 128);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, true);

        super::inc(&mut a, &mut p);
        assert_eq!(a, 129);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, true);

        a = 254;

        super::inc(&mut a, &mut p);
        assert_eq!(a, 255);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, true);

        super::inc(&mut a, &mut p);
        assert_eq!(a, 0);
        assert_eq!(p.Z, true);
        assert_eq!(p.N, false);
    }

    #[test]
    fn dec() {
        let mut a: u8 = 129;
        let mut p = RegisterP::default();

        super::dec(&mut a, &mut p);
        assert_eq!(a, 128);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, true);

        super::dec(&mut a, &mut p);
        assert_eq!(a, 127);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, false);

        super::dec(&mut a, &mut p);
        assert_eq!(a, 126);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, false);

        a = 2;

        super::dec(&mut a, &mut p);
        assert_eq!(a, 1);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, false);

        super::dec(&mut a, &mut p);
        assert_eq!(a, 0);
        assert_eq!(p.Z, true);
        assert_eq!(p.N, false);

        super::dec(&mut a, &mut p);
        assert_eq!(a, 255);
        assert_eq!(p.Z, false);
        assert_eq!(p.N, true);
    }

    #[duplicate_item(
        DUP_name            DUP_op      DUP_a       DUP_res     DUP_z;
        [tsb_no_change]     [0b00110011][0b00110011][0b00110011][false];
        [tsb_set_all]       [0b00000000][0b11111111][0b11111111][true];
        [tsb_set_half]      [0b10100000][0b00001111][0b10101111][true];
        [tsb_set_half2]     [0b10100001][0b00001111][0b10101111][false];
        [tsb_set_nothing]   [0b10100001][0b00000000][0b10100001][true];
    )]
    #[test]
    fn DUP_name() {
        let a: u8 = DUP_a;
        let mut op: u8 = DUP_op;
        let mut p = RegisterP::default();

        super::tsb(&mut op, a, &mut p);

        assert_eq!(p.Z, DUP_z);
        assert_eq!(op, DUP_res);
    }

    #[duplicate_item(
        DUP_name            DUP_op      DUP_a       DUP_res     DUP_z;
        [trb_unset]         [0b00110011][0b00110011][0b00000000][false];
        [trb_unset_all]     [0b11111111][0b11111111][0b00000000][false];
        [trb_no_change]     [0b10100000][0b00001111][0b10100000][true];
        [trb_change1]       [0b10100001][0b00001111][0b10100000][false];
        [trb_unset_nothing] [0b10100001][0b00000000][0b10100001][true];
    )]
    #[test]
    fn DUP_name() {
        let a: u8 = DUP_a;
        let mut op: u8 = DUP_op;
        let mut p = RegisterP::default();

        super::trb(&mut op, a, &mut p);

        assert_eq!(p.Z, DUP_z);
        assert_eq!(op, DUP_res);
    }
}

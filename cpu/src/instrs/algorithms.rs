use crate::reg::Reg;
use crate::registers::RegisterP;

fn set_nz<T: Reg>(val: T, p: &mut RegisterP) {
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
}

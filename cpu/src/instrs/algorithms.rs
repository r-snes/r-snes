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

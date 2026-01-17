use duplicate::{duplicate_item, duplicate};
use instr_metalang_procmacro::{cpu_instr, var_width_op};
use super::prelude::*;

// useful trait in code that will be duplicated for u8 and u16 operands
// providing the necessary checks for setting N and Z flags
trait Num {
    fn is_zero(&self) -> bool;
    fn is_neg(&self) -> bool;
}

impl Num for u8 {
    fn is_zero(&self) -> bool {
        *self == 0
    }
    fn is_neg(&self) -> bool {
        *self > 0x7f
    }
}
impl Num for u16 {
    fn is_zero(&self) -> bool {
        *self == 0
    }
    fn is_neg(&self) -> bool {
        *self > 0x7fff
    }
}

fn set_nz<T: Num>(val: T, cpu: &mut CPU) {
    cpu.registers.P.Z = val.is_zero();
    cpu.registers.P.N = val.is_neg();
}

pub(crate) trait VarWidthOp {
    fn op8(cpu: &mut CPU);
    fn op16(cpu: &mut CPU);
}

struct AND {}
impl VarWidthOp for AND {
    #[var_width_op(a = cpu.registers.A, idb = cpu.internal_data_bus)]
    fn op(cpu: &mut CPU) {
        *a &= *idb;
        set_nz(*a, cpu);
    }
}

struct EOR {}
impl VarWidthOp for EOR {
    #[var_width_op(a = cpu.registers.A, idb = cpu.internal_data_bus)]
    fn op(cpu: &mut CPU) {
        *a ^= *idb;
        set_nz(*a, cpu);
    }
}

struct ORA {}
impl VarWidthOp for ORA {
    #[var_width_op(a = cpu.registers.A, idb = cpu.internal_data_bus)]
    fn op(cpu: &mut CPU) {
        *a |= *idb;
        set_nz(*a, cpu);
    }
}

// CMP, CPX and CPY all use the same algorithm: compare an operand
// against a register, so we duplicate the implementations
struct CMP {}
struct CPX {}
struct CPY {}
duplicate! {
    [
        DUP_name    DUP_reg;
        [CMP]       [A];
        [CPX]       [X];
        [CPY]       [Y];
    ]
    impl VarWidthOp for DUP_name {
        #[var_width_op(reg = cpu.registers.DUP_reg, idb = cpu.internal_data_bus)]
        fn op(cpu: &mut CPU) {
            let (diff, overflow) = reg.overflowing_sub(*idb);

            cpu.registers.P.Z = diff == 0;
            cpu.registers.P.C = !overflow;
            cpu.registers.P.N = diff.is_neg();
        }
    }
}

struct ADC {}
impl VarWidthOp for ADC {
    #[var_width_op(a = cpu.registers.A, idb = cpu.internal_data_bus)]
    fn op(cpu: &mut CPU) {
        if !cpu.registers.P.D {
            let (res, carry_out) = a.carrying_add(*idb, cpu.registers.P.C);

            cpu.registers.P.C = carry_out;
            cpu.registers.P.V = ((*a ^ res) & (*idb ^ res)).is_neg();
            *a = res;
            set_nz(*a, cpu);
        } else {
            todo!("decimal mode is not supported yet");
        }
    }
}

struct SBC {}
impl VarWidthOp for SBC {
    #[var_width_op(a = cpu.registers.A, idb = cpu.internal_data_bus)]
    fn op(cpu: &mut CPU) {
        *idb = !*idb; // negate the operand and then perform addition (a - b = a + (-b))

        // same as addition
        if !cpu.registers.P.D {
            let (res, carry_out) = a.carrying_add(*idb, cpu.registers.P.C);

            cpu.registers.P.C = carry_out;
            cpu.registers.P.V = ((*a ^ res) & (*idb ^ res)).is_neg();
            *a = res;
            set_nz(*a, cpu);
        } else {
            todo!("decimal mode is not supported yet");
        }
    }
}

// duplicate over these 6 instructions which share the same 15 addressing modes
#[duplicate_item(
    DUP1_name   DUP1_algo;
    [eor]       [EOR];
    [ora]       [ORA];
    [and]       [AND];
    [cmp]       [CMP];
    [adc]       [ADC];
    [sbc]       [SBC];
)]
pub(crate) mod DUP1_name {
    use super::*;
    // we use a typedef instead of using DUP1_algo directly because it
    // doesn't substitute properly in the nested `duplicate!` call below
    type Algo = DUP1_algo;

    // duplicate for all addressing modes of these instructions
    duplicate! {
        [
            DUP2_name   DUP2_addrmode;
            [abs]       [SET_ADDRMODE_ABS];
            [absl]      [SET_ADDRMODE_ABSL];
            [abslx]     [SET_ADDRMODE_ABSLX];
            [absx]      [SET_ADDRMODE_ABSX];
            [absy]      [SET_ADDRMODE_ABSY];
            [d]         [SET_ADDRMODE_DIRECT];
            [dxind]     [SET_ADDRMODE_DIRECTX_IND];
            [dind]      [SET_ADDRMODE_DIRECT_IND];
            [dindy]     [SET_ADDRMODE_DIRECT_INDY];
            [dindly]    [SET_ADDRMODE_DIRECT_INDLY];
            [dindl]     [SET_ADDRMODE_DIRECT_INDL];
            [dx]        [SET_ADDRMODE_DIRECTX];
            [imm]       [SET_ADDRMODE_IMM];
            [sr]        [SET_ADDRMODE_STACKREL];
            [sry]       [SET_ADDRMODE_STACKREL_INDY];
        ]
        cpu_instr!(DUP2_name {
            meta SET_OP_SIZE AccMem;
            meta DUP2_addrmode;

            meta FETCH_OP_INTO cpu.internal_data_bus;
            meta VAR_WIDTH_OP Algo;
        });
    }
}

// duplicate for CPX and CPY, which are similar to the above except
// they exist for fewer addrmodes and their width depend on the X flag
duplicate! {
    [
        DUP_name    DUP_algo    DUP_addrmode;
        [cpx_imm]   [CPX]       [SET_ADDRMODE_IMM];
        [cpx_abs]   [CPX]       [SET_ADDRMODE_ABS];
        [cpx_d]     [CPX]       [SET_ADDRMODE_DIRECT];

        [cpy_imm]   [CPY]       [SET_ADDRMODE_IMM];
        [cpy_abs]   [CPY]       [SET_ADDRMODE_ABS];
        [cpy_d]     [CPY]       [SET_ADDRMODE_DIRECT];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE Index;
        meta DUP_addrmode;

        meta FETCH_OP_INTO cpu.internal_data_bus;
        meta VAR_WIDTH_OP DUP_algo;
    });
}

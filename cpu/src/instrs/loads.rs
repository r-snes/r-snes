use duplicate::duplicate;
use instr_metalang_procmacro::cpu_instr;

duplicate! {
    [
        DUP_name    DUP_addrmode;
        [ldx_imm]   [SET_ADDRMODE_IMM];
        [ldx_abs]   [SET_ADDRMODE_ABS];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE Index;

        meta DUP_addrmode;
        meta FETCH_OP_INTO cpu.registers.X;
        meta SET_NZ_OP cpu.registers.X;
    });
}

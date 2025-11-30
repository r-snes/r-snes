use duplicate::duplicate;
use instr_metalang_procmacro::cpu_instr;

duplicate! {
    [
        DUP_name    DUP_dest    DUP_opsize  DUP_addrmode;
        [ldx_imm]   [X]         [Index]     [SET_ADDRMODE_IMM];
        [ldx_abs]   [X]         [Index]     [SET_ADDRMODE_ABS];
        [ldx_absy]  [X]         [Index]     [SET_ADDRMODE_ABSY];
        [ldx_d]     [X]         [Index]     [SET_ADDRMODE_DIRECT];
        [ldx_dy]    [X]         [Index]     [SET_ADDRMODE_DIRECTY];

        [ldy_imm]   [Y]         [Index]     [SET_ADDRMODE_IMM];
        [ldy_abs]   [Y]         [Index]     [SET_ADDRMODE_ABS];
        [ldy_absx]  [Y]         [Index]     [SET_ADDRMODE_ABSX];
        [ldy_d]     [Y]         [Index]     [SET_ADDRMODE_DIRECT];
        [ldy_dx]    [Y]         [Index]     [SET_ADDRMODE_DIRECTX];

        [lda_imm]   [A]         [AccMem]    [SET_ADDRMODE_IMM];
        [lda_abs]   [A]         [AccMem]    [SET_ADDRMODE_ABS];
        [lda_absl]  [A]         [AccMem]    [SET_ADDRMODE_ABSL];
        [lda_abslx] [A]         [AccMem]    [SET_ADDRMODE_ABSLX];
        [lda_absx]  [A]         [AccMem]    [SET_ADDRMODE_ABSX];
        [lda_absy]  [A]         [AccMem]    [SET_ADDRMODE_ABSY];
        [lda_d]     [A]         [AccMem]    [SET_ADDRMODE_DIRECT];
        [lda_dxind] [A]         [AccMem]    [SET_ADDRMODE_DIRECTX_IND];
        [lda_dind]  [A]         [AccMem]    [SET_ADDRMODE_DIRECT_IND];
        [lda_dindy] [A]         [AccMem]    [SET_ADDRMODE_DIRECT_INDY];
        [lda_dindly][A]         [AccMem]    [SET_ADDRMODE_DIRECT_INDLY];
        [lda_dindl] [A]         [AccMem]    [SET_ADDRMODE_DIRECT_INDL];
        [lda_dx]    [A]         [AccMem]    [SET_ADDRMODE_DIRECTX];
        [lda_sr]    [A]         [AccMem]    [SET_ADDRMODE_STACKREL];
        [lda_sry]   [A]         [AccMem]    [SET_ADDRMODE_STACKREL_INDY];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_opsize;

        meta DUP_addrmode;
        meta FETCH_OP_INTO cpu.registers.DUP_dest;
        meta SET_NZ_OP cpu.registers.DUP_dest;
    });
}

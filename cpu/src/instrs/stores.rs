use instr_metalang_procmacro::cpu_instr;
use duplicate::duplicate;

duplicate! {
    [
        DUP_name    DUP_src             DUP_opsize  DUP_addrmode;
        // stx: write X register to memory
        [stx_abs]   [cpu.registers.X]   [Index]     [SET_ADDRMODE_ABS];
        [stx_d]     [cpu.registers.X]   [Index]     [SET_ADDRMODE_DIRECT];
        [stx_dy]    [cpu.registers.X]   [Index]     [SET_ADDRMODE_DIRECTY];

        // sty: write Y register to memory
        [sty_abs]   [cpu.registers.Y]   [Index]     [SET_ADDRMODE_ABS];
        [sty_d]     [cpu.registers.Y]   [Index]     [SET_ADDRMODE_DIRECT];
        [sty_dx]    [cpu.registers.Y]   [Index]     [SET_ADDRMODE_DIRECTX];

        // sta: write A register to memory
        [sta_abs]   [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABS];
        [sta_absl]  [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABSL];
        [sta_abslx] [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABSLX];
        [sta_absx]  [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABSX];
        [sta_absy]  [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_ABSY];
        [sta_d]     [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT];
        [sta_dxind] [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECTX_IND];
        [sta_dind]  [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT_IND];
        [sta_dindy] [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT_INDY];
        [sta_dindly][cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT_INDLY];
        [sta_dindl] [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECT_INDL];
        [sta_dx]    [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_DIRECTX];
        [sta_sr]    [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_STACKREL];
        [sta_sry]   [cpu.registers.A]   [AccMem]    [SET_ADDRMODE_STACKREL_INDY];

        // stz: write 0 to memory
        [stz_abs]   [0]                 [AccMem]    [SET_ADDRMODE_ABS];
        [stz_absx]  [0]                 [AccMem]    [SET_ADDRMODE_ABSX];
        [stz_d]     [0]                 [AccMem]    [SET_ADDRMODE_DIRECT];
        [stz_dx]    [0]                 [AccMem]    [SET_ADDRMODE_DIRECTX];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_opsize;

        meta DUP_addrmode;
        meta WRITE_OP DUP_src;
    });
}

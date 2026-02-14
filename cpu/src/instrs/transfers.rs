//! Module which defines "transfer" instructions: simple instructions
//! which copy the content of one register to another
//!
//! For all of those instructions, the size (8 bit vs 16 bit) of the
//! transfer is determined by the size of the destination register.
//! The name of the instruction mnenomics means 'Transfer'-'Source'-'Dest',
//! for example TXS means 'Transfer X to S'.

use super::algorithms;
use instr_metalang_procmacro::cpu_instr;
use duplicate::duplicate;

// duplicate over all transfers to S, because S becomes 8 bit
// in emu mode
duplicate! {
    [
        DUP_name    DUP_src;
        [txs]       [X];
        [tcs]       [A];
    ]
    cpu_instr!(DUP_name {
        if cpu.registers.E {
            *cpu.registers.S.lo_mut() = *cpu.registers.DUP_src.lo();
        } else {
            cpu.registers.S = cpu.registers.DUP_src;
        }

        meta END_CYCLE Internal;
    });
}

// duplicate over variable width registers (as destination --
// since the size of the transfer comes from the size of the source)
duplicate! {
    [
        DUP_name    DUP_src     DUP_dest    DUP_size;
        [tax]       [A]         [X]         [Index];
        [tay]       [A]         [Y]         [Index];
        [tsx]       [S]         [X]         [Index];
        [txa]       [X]         [A]         [AccMem];
        [txy]       [X]         [Y]         [Index];
        [tya]       [Y]         [A]         [AccMem];
        [tyx]       [Y]         [X]         [Index];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_size;

        meta LET_VARWIDTH_MUT dest = cpu.registers.DUP_dest;
        meta LET_VARWIDTH src = cpu.registers.DUP_src;

        *dest = src;

        algorithms::set_nz(*dest, &mut cpu.registers.P);

        meta END_CYCLE Internal;
    });
}

// duplicate over all 16-bit transfers
duplicate! {
    [
        DUP_name    DUP_src     DUP_dest;
        [tcd]       [A]         [D];
        [tdc]       [D]         [A];
        [tsc]       [S]         [A];
    ]
    cpu_instr!(DUP_name {
        cpu.registers.DUP_dest = cpu.registers.DUP_src;
        algorithms::set_nz(cpu.registers.DUP_dest, &mut cpu.registers.P);

        meta END_CYCLE Internal;
    });
}

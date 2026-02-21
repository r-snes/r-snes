use instr_metalang_procmacro::cpu_instr;
use duplicate::duplicate;

duplicate! {
    [
        DUP_name    DUP_reg     DUP_size;
        [pha]       [A]         [AccMem];
        [phx]       [X]         [Index];
        [phy]       [Y]         [Index];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_size;

        meta END_CYCLE Internal;

        meta PUSH_OP cpu.registers.DUP_reg;
    });
}

duplicate! {
    [
        DUP_name    DUP_reg     DUP_size;
        [pla]       [A]         [AccMem];
        [plx]       [X]         [Index];
        [ply]       [Y]         [Index];
    ]
    cpu_instr!(DUP_name {
        meta SET_OP_SIZE DUP_size;

        meta END_CYCLE Internal;
        meta END_CYCLE Internal;

        meta PULL_OP_INTO cpu.registers.DUP_reg;
        meta SET_NZ_OP cpu.registers.DUP_reg;
    });
}

cpu_instr!(php {
    meta END_CYCLE Internal;
    meta PUSH8 cpu.registers.P.into();
});

cpu_instr!(phd {
    meta END_CYCLE Internal;
    meta PUSHN16 cpu.registers.D;
});

cpu_instr!(phb {
    meta END_CYCLE Internal;
    meta PUSHN8 cpu.registers.DB;
});

cpu_instr!(phk {
    meta END_CYCLE Internal;
    meta PUSHN8 cpu.registers.PB;
});

cpu_instr!(plb {
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULLN8_INTO cpu.registers.DB;
    meta SET_NZ8 cpu.registers.DB;
});

cpu_instr!(pld {
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULLN16_INTO cpu.registers.D;
    meta SET_NZ16 cpu.registers.D;
});

cpu_instr!(plp {
    meta END_CYCLE Internal;
    meta END_CYCLE Internal;

    meta PULL8;
    cpu.registers.P = cpu.data_bus.into();

    if cpu.registers.E {
        cpu.registers.P.M = true;
        cpu.registers.P.X = true;
    } else {
        if cpu.registers.P.X {
            *cpu.registers.X.hi_mut() = 0;
            *cpu.registers.Y.hi_mut() = 0;
        }
    }
});

cpu_instr!(pea {
    meta FETCH16_IMM_INTO cpu.internal_data_bus;
    meta PUSHN16 cpu.internal_data_bus;
});

cpu_instr!(per {
    meta FETCH16_IMM_INTO cpu.internal_data_bus;

    cpu.internal_data_bus = cpu.registers.PC
        .wrapping_add(3)
        .wrapping_add(cpu.internal_data_bus);

    meta END_CYCLE Internal;

    meta PUSHN16 cpu.internal_data_bus;
});

cpu_instr!(pei {
    meta SET_ADDRMODE_DIRECT;

    meta FETCH16_INTO cpu.internal_data_bus;
    meta PUSHN16 cpu.internal_data_bus;
});

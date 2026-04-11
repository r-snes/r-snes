use instr_metalang_procmacro::cpu_instr_no_inc_pc;
use duplicate::duplicate;

cpu_instr_no_inc_pc!(brk {
    meta FETCH8_IMM; // ignored imm read
    
    if cpu.registers.E {
        // skip the PB push if in emu mode
        return brk_cyc3(cpu);
    }
    meta PUSH8 cpu.registers.PB;
    meta PUSH16 cpu.registers.PC.wrapping_add(2);
    meta PUSH8 cpu.registers.P.into();

    cpu.registers.P.I = true;
    cpu.registers.P.D = false;

    let addr = if cpu.registers.E {
        0xFFFE
    } else {
        0xFFE6
    };
    cpu.addr_bus = snes_addr!(0:addr);
    meta FETCH16_INTO cpu.registers.PC;
    cpu.registers.PB = 0;
});

cpu_instr_no_inc_pc!(cop {
    meta FETCH8_IMM; // ignored imm read
    
    if cpu.registers.E {
        // skip the PB push if in emu mode
        return cop_cyc3(cpu);
    }
    meta PUSH8 cpu.registers.PB;
    meta PUSH16 cpu.registers.PC.wrapping_add(2);
    meta PUSH8 cpu.registers.P.into();

    cpu.registers.P.I = true;
    cpu.registers.P.D = false;

    let addr = if cpu.registers.E {
        0xFFF4
    } else {
        0xFFE4
    };
    cpu.addr_bus = snes_addr!(0:addr);
    meta FETCH16_INTO cpu.registers.PC;
    cpu.registers.PB = 0;
});

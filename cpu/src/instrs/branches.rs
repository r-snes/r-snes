use instr_metalang_procmacro::cpu_instr_no_inc_pc;
use duplicate::duplicate;

duplicate! {
    [
        DUP_name    DUP_flag;
        [bcs]       [ cpu.registers.P.C]; // Branch if Carry Set
        [bcc]       [!cpu.registers.P.C]; // Branch if Carry Clear
        [beq]       [ cpu.registers.P.Z]; // Branch if EQual
        [bne]       [!cpu.registers.P.Z]; // Branch if Not Equal
        [bmi]       [ cpu.registers.P.N]; // Branch if MInus
        [bpl]       [!cpu.registers.P.N]; // Branch if PLus
        [bvs]       [ cpu.registers.P.V]; // Branch if oVerflow Set
        [bvc]       [!cpu.registers.P.V]; // Branch if oVerflow Clear
        [bra]       [true]; // BRanch Always
    ]
    cpu_instr_no_inc_pc!(DUP_name {
        meta FETCH8_IMM;

        // manually inc PC to where it would be for the next opcode
        cpu.registers.PC = cpu.registers.PC.wrapping_add(2);

        meta IDLE_IF DUP_flag; // idle if the branch is taken (cpu doc note 5)
        if DUP_flag {
            // when branching, save old PC, before overwriting to check page boundary crossing
            cpu.internal_data_bus = cpu.registers.PC;
            // offset PC by the read value as a signed number
            cpu.registers.PC = cpu.registers.PC.wrapping_add(cpu.data_bus as i8 as u16);
        }

        // idle if the branch is taken across a page boundary (cpu doc note 6)
        meta IDLE_IF DUP_flag
            && cpu.registers.P.E
            && *cpu.internal_data_bus.hi() != *cpu.registers.PC.hi();
    });
}

// BRanch Long (unconditionally)
cpu_instr_no_inc_pc!(brl {
    meta FETCH16_INTO cpu.internal_data_bus;

    cpu.registers.PC = cpu.registers.PC.wrapping_add(cpu.internal_data_bus);
    meta END_CYCLE Internal;
});

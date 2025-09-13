use apu::Apu;
use apu::cpu::{FLAG_N, FLAG_Z};

fn main() {
    let mut apu = Apu::new();

    // Program:
    // 0x0200: LDA #$42
    // 0x0202: STA $0010
    // 0x0204: MOV A, X
    // 0x0205: NOP
    // 0x0206: BRK (0xFF) - stop execution
    apu.memory.write8(0x0200, 0xA9); // LDA #imm
    apu.memory.write8(0x0201, 0x42); // operand
    apu.memory.write8(0x0202, 0x85); // STA dp
    apu.memory.write8(0x0203, 0x10); // direct page $0010
    apu.memory.write8(0x0204, 0xE8); // MOV A, X
    apu.memory.write8(0x0205, 0x00); // NOP
    apu.memory.write8(0x0206, 0xFF); // BRK - stop

    // Initialize registers
    apu.cpu.regs.pc = 0x0200;
    apu.cpu.regs.x = 0x99;

    println!("Starting CPU execution:");
    println!(
        "Initial state: PC={:04X} A={:02X} X={:02X} Y={:02X} SP={:02X} Cycles={}",
        apu.cpu.regs.pc, apu.cpu.regs.a, apu.cpu.regs.x, apu.cpu.regs.y,
        apu.cpu.regs.sp, apu.cpu.cycles
    );

    loop {
        let pc = apu.cpu.regs.pc;
        let opcode = apu.memory.read8(pc);

        // Fetch operand if relevant
        let operand = match opcode {
            0xA9 | 0xA2 | 0xA0 => apu.memory.read8(pc + 1) as u16, // immediate
            0x85 | 0x86 | 0x87 | 0xA5 | 0xA6 | 0xA7 => apu.memory.read8(pc + 1) as u16, // direct page
            0x8D | 0x8E | 0x8F | 0xAD | 0xAE | 0xAF => apu.memory.read16(pc), // absolute
            _ => 0,
        };

        // Execute instruction
        apu.step(1);

        // Trace
        println!("\nPC={:04X} Opcode {:02X} Operand={:04X}", pc, opcode, operand);
        println!(
            "Registers: A={:02X} X={:02X} Y={:02X} SP={:02X} PC={:04X}",
            apu.cpu.regs.a, apu.cpu.regs.x, apu.cpu.regs.y,
            apu.cpu.regs.sp, apu.cpu.regs.pc
        );
        println!(
            "Flags: N={} Z={}",
            apu.cpu.get_flag(FLAG_N),
            apu.cpu.get_flag(FLAG_Z)
        );
        println!("Total cycles: {}", apu.cpu.cycles);

        // Show memory writes (direct page stores)
        if matches!(opcode, 0x85 | 0x86 | 0x87) {
            println!(
                "Memory[0x{:04X}] <- {:02X}",
                operand, apu.memory.read8(operand)
            );
        }

        // Stop execution if BRK
        if opcode == 0xFF {
            println!("\nBRK encountered. CPU halted.");
            break;
        }
    }

    println!("\nFinal state:");
    println!(
        "PC={:04X} A={:02X} X={:02X} Y={:02X} SP={:02X} Cycles={}",
        apu.cpu.regs.pc, apu.cpu.regs.a, apu.cpu.regs.x, apu.cpu.regs.y,
        apu.cpu.regs.sp, apu.cpu.cycles
    );
}

//What this does:
// Prints initial state of the CPU before running.
// Before each step(), it fetches and shows the opcode at the current PC.
// After the step, prints all registers, relevant flags, and total cycles.
// Loops for a set number of steps (5 here), so you can see the CPU run instruction by instruction.
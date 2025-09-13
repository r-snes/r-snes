use apu::Apu;
use apu::cpu::{FLAG_N, FLAG_Z};

fn main() {
    let mut apu = Apu::new();

    // Program memory setup
    // 0x0200: LDA #$42      -> load 0x42 into A
    // 0x0202: STA $0010      -> store A into direct page 0x10
    // 0x0204: MOV A, X       -> copy X into A
    // 0x0205: NOP
    // 0x0206: ADC #$01       -> add 1 to A
    // 0x0208: CMP #$42       -> compare A with 0x42
    // 0x020A: AND #$0F       -> A = A & 0x0F
    // 0x020C: ORA #$F0       -> A = A | 0xF0
    // 0x020E: EOR #$FF       -> A = A ^ 0xFF
    // 0x0210: BRK (stop)
    apu.memory.write8(0x0200, 0xA9); // LDA #imm
    apu.memory.write8(0x0201, 0x42); // operand
    apu.memory.write8(0x0202, 0xC4); // MOV d, A (our STA replacement)
    apu.memory.write8(0x0203, 0x10); // direct page $10
    apu.memory.write8(0x0204, 0x7D); // MOV A, X
    apu.memory.write8(0x0205, 0x00); // NOP
    apu.memory.write8(0x0206, 0x69); // ADC #imm
    apu.memory.write8(0x0207, 0x01); // operand
    apu.memory.write8(0x0208, 0xC9); // CMP #imm
    apu.memory.write8(0x0209, 0x42); // operand
    apu.memory.write8(0x020A, 0x29); // AND #imm
    apu.memory.write8(0x020B, 0x0F); // operand
    apu.memory.write8(0x020C, 0x09); // ORA #imm
    apu.memory.write8(0x020D, 0xF0); // operand
    apu.memory.write8(0x020E, 0x49); // EOR #imm
    apu.memory.write8(0x020F, 0xFF); // operand
    apu.memory.write8(0x0210, 0x00); // BRK (stop)

    // Initialize CPU registers
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

        // Fetch operand for logging
        let operand = match opcode {
            0xA9 | 0xCD | 0x8D | 0x69 | 0xC9 | 0xE9 | 0x29 | 0x09 | 0x49 => {
                apu.memory.read8(pc + 1) as u16
            }
            0xC4 => apu.memory.read8(pc + 1) as u16, // direct page store
            _ => 0,
        };

        // Execute instruction
        apu.step(1);

        // Log step
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

        // Show memory writes for direct page stores
        if opcode == 0xC4 {
            println!(
                "Memory[0x{:04X}] <- {:02X}",
                operand, apu.memory.read8(operand)
            );
        }

        // Stop execution at BRK
        if opcode == 0x00 || opcode == 0xFF {
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
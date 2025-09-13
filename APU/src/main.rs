use apu::Apu;
use apu::cpu::{FLAG_N, FLAG_Z};

fn main() {
    let mut apu = Apu::new();

    // Load a small program into RAM:
    // 0x0200: MOV A, X (0xE8)
    // 0x0201: NOP (0x00)
    apu.memory.write8(0x0200, 0xE8);
    apu.memory.write8(0x0201, 0x00);

    // Set program counter and initial registers
    apu.cpu.regs.pc = 0x0200;
    apu.cpu.regs.x = 0x42;

    println!("Starting CPU execution:");
    println!("Initial state: PC={:04X} A={:02X} X={:02X} Cycles={}",
        apu.cpu.regs.pc, apu.cpu.regs.a, apu.cpu.regs.x, apu.cpu.cycles);

    for step in 0..5 {
        // Fetch the next opcode to display
        let pc = apu.cpu.regs.pc;
        let opcode = apu.memory.read8(pc);

        // Execute one instruction
        apu.step(1);

        // Display what happened
        println!("\nStep {}:", step + 1);
        println!("Executed opcode {:02X} at PC={:04X}", opcode, pc);
        println!("Registers after step: A={:02X} X={:02X} Y={:02X} SP={:02X} PC={:04X}",
            apu.cpu.regs.a, apu.cpu.regs.x, apu.cpu.regs.y,
            apu.cpu.regs.sp, apu.cpu.regs.pc);

        // Display flags
        println!(
            "Flags: N={} Z={}",
            apu.cpu.get_flag(FLAG_N),
            apu.cpu.get_flag(FLAG_Z)
        );

        println!("Total cycles: {}", apu.cpu.cycles);
    }

    println!("\nExecution finished.");
}

//What this does:
// Prints initial state of the CPU before running.
// Before each step(), it fetches and shows the opcode at the current PC.
// After the step, prints all registers, relevant flags, and total cycles.
// Loops for a set number of steps (5 here), so you can see the CPU run instruction by instruction.
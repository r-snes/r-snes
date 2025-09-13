use apu::Apu;

fn main() {
    let mut apu = Apu::new();

    apu.memory.write8(0x0200, 0xE8);
    apu.memory.write8(0x0201, 0xFF);

    apu.cpu.regs.pc = 0x0200;
    apu.cpu.regs.x = 0x42;

    for _ in 0..5 {
        apu.step(1);
        println!(
            "PC={:04X} A={:02X} X={:02X} Cycles={}",
            apu.cpu.regs.pc,
            apu.cpu.regs.a,
            apu.cpu.regs.x,
            apu.cpu.cycles
        );
    }
}

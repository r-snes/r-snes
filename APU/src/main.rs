mod spc700;

use spc700::Spc700;

fn main() {
    let mut cpu = Spc700::new();
    cpu.reset();

    cpu.write_byte(0xfffe, 0x00); // Set reset vector to 0x0000
    cpu.write_byte(0xffff, 0x00);

    cpu.write_byte(0x0000, 0xe8); // MOV A, #0x42
    cpu.write_byte(0x0001, 0x42);
    cpu.write_byte(0x0002, 0x00); // NOP

    for _ in 0..3 {
        cpu.step();
    }

    println!("Accumulator value: 0x{:02X}", cpu.a);
}

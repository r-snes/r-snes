mod spc700;
mod dsp;

use spc700::Spc700;

fn main()
{
    let mut cpu = Spc700::new();

    cpu.write_byte(0xfffe, 0x00); // Set reset vector to 0x0000
    cpu.write_byte(0xffff, 0x00);

    // Program:
    cpu.write_byte(0x0000, 0xE8); // MOV A, #0x2C  (V0_PITCHL select)
    cpu.write_byte(0x0001, 0x2C);
    cpu.write_byte(0x0002, 0xC4); // MOV ($F2), A
    cpu.write_byte(0x0003, 0xF2);

    // Set low pitch
    cpu.write_byte(0x0004, 0xE8); // MOV A, #0x00 (low byte)
    cpu.write_byte(0x0005, 0x00);
    cpu.write_byte(0x0006, 0xC4); // MOV ($F3), A
    cpu.write_byte(0x0007, 0xF3);

    // Select high pitch
    cpu.write_byte(0x0008, 0xE8); // MOV A, #0x2D (V0_PITCHH select)
    cpu.write_byte(0x0009, 0x2D);
    cpu.write_byte(0x000A, 0xC4); // MOV ($F2), A
    cpu.write_byte(0x000B, 0xF2);

    // Set high pitch
    cpu.write_byte(0x000C, 0xE8); // MOV A, #0x10 (high byte)
    cpu.write_byte(0x000D, 0x10);
    cpu.write_byte(0x000E, 0xC4); // MOV ($F3), A
    cpu.write_byte(0x000F, 0xF3);

    // Set left volume
    cpu.write_byte(0x0010, 0xE8); // MOV A, #0x0C (V0_VOL_LEFT select)
    cpu.write_byte(0x0011, 0x0C);
    cpu.write_byte(0x0012, 0xC4); // MOV ($F2), A
    cpu.write_byte(0x0013, 0xF2);

    cpu.write_byte(0x0014, 0xE8); // MOV A, #0x7F (max volume)
    cpu.write_byte(0x0015, 0x7F);
    cpu.write_byte(0x0016, 0xC4); // MOV ($F3), A
    cpu.write_byte(0x0017, 0xF3);

    // Set right volume
    cpu.write_byte(0x0018, 0xE8); // MOV A, #0x1C (V0_VOL_RIGHT select)
    cpu.write_byte(0x0019, 0x1C);
    cpu.write_byte(0x001A, 0xC4); // MOV ($F2), A
    cpu.write_byte(0x001B, 0xF2);

    cpu.write_byte(0x001C, 0xE8); // MOV A, #0x7F (max volume)
    cpu.write_byte(0x001D, 0x7F);
    cpu.write_byte(0x001E, 0xC4); // MOV ($F3), A
    cpu.write_byte(0x001F, 0xF3);

    // KEY ON voice 0
    cpu.write_byte(0x0020, 0xE8); // MOV A, #0x4C (KEYON select)
    cpu.write_byte(0x0021, 0x4C);
    cpu.write_byte(0x0022, 0xC4); // MOV ($F2), A
    cpu.write_byte(0x0023, 0xF2);

    cpu.write_byte(0x0024, 0xE8); // MOV A, #0x01 (bit 0 = voice 0 ON)
    cpu.write_byte(0x0025, 0x01);
    cpu.write_byte(0x0026, 0xC4); // MOV ($F3), A
    cpu.write_byte(0x0027, 0xF3);

    cpu.reset();

    for _ in 0..100 {
        cpu.step();
        cpu.dsp.step();
    }

    println!("Test done.");
}

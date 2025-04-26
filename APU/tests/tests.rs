use APU::dsp::Dsp;
use APU::spc700::Spc700;

#[cfg(test)]
mod tests {
    use super::*; // Import the top-level module to access your code

    // Test for writing to a register and reading it back
    #[test]
    fn test_dsp_register_write_and_read() {
        let mut dsp = Dsp::new(); // Initialize your Dsp object
        dsp.set_register_select(0x2C); // Set register to 0x2C
        dsp.write_selected_register(0x30); // Write value 0x30 to register 0x2C

        let read_value = dsp.read_selected_register(); // Read back the value
        assert_eq!(read_value, 0x30, "DSP: Incorrect value read from register 0x2C");
    }

    // Test for updating pitch of voice 0 in the DSP
    #[test]
    fn test_voice0_pitch_update() {
        let mut dsp = Dsp::new();
        dsp.set_register_select(0x2C);
        dsp.write_selected_register(0x30); // Write the low byte of pitch
        dsp.set_register_select(0x2D);
        dsp.write_selected_register(0x12); // Write the high byte of pitch

        assert_eq!(dsp.voice0_pitch, 0x1230, "Voice 0 pitch is incorrect");
    }

    // Test if DSP step function works correctly
    #[test]
    fn test_dsp_step_function() {
        let mut dsp = Dsp::new();

        // Set up the registers for a typical DSP update cycle
        dsp.set_register_select(0x4C);
        dsp.write_selected_register(0x01); // Voice 0 key on

        dsp.set_register_select(0x2C);
        dsp.write_selected_register(0x30); // Low byte of pitch

        dsp.set_register_select(0x2D);
        dsp.write_selected_register(0x12); // High byte of pitch

        dsp.set_register_select(0x4C);
        dsp.write_selected_register(0x50); // Left volume

        dsp.set_register_select(0x5C);
        dsp.write_selected_register(0x60); // Right volume

        dsp.step(); // Call the DSP step function

        // Assertions to check if the state has been updated correctly
        assert_eq!(dsp.voice0_pitch, 0x1230, "Voice 0 pitch is incorrect");
        assert_eq!(dsp.voice0_volume_left, 0x50, "Voice 0 left volume is incorrect");
        assert_eq!(dsp.voice0_volume_right, 0x60, "Voice 0 right volume is incorrect");
    }

    // Test the Spc700 functionality (assuming a similar structure exists for Spc700)
    #[test]
    fn test_spc700_memory_read_and_write() {
        let mut spc700 = Spc700::new();
    
        spc700.write_byte(0x0010, 0xFF); // Write 0xFF into RAM address 0x10
        let value = spc700.read_byte(0x0010); // Read it back
    
        assert_eq!(value, 0xFF, "Spc700: Incorrect value read from memory address 0x10");
    }

    // Add additional tests based on other methods you need to cover.
}

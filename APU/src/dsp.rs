pub struct Dsp {
    pub regs: [u8; 128], // 128 DSP registers
    pub selected: u8,    // currently selected register
    
    // Voice 0 state:
    pub voice0_pitch: u16,
    pub voice0_volume_left: u8,
    pub voice0_volume_right: u8,
    pub voice0_key_on: bool,
}

impl Dsp {
    pub fn new() -> Self {
        Dsp {
            regs: [0; 128],
            selected: 0,
            voice0_pitch: 0,
            voice0_volume_left: 0,
            voice0_volume_right: 0,
            voice0_key_on: false,
        }
    }

    pub fn set_register_select(&mut self, reg: u8) {
        self.selected = reg & 0x7F; // only lower 7 bits are valid
    }

    pub fn write_selected_register(&mut self, value: u8) {
        println!("DSP: Wrote 0x{:02X} to register 0x{:02X}", value, self.selected);
    
        self.regs[self.selected as usize] = value;
    
        match self.selected {
            0x2C => {
                println!("Setting Voice 0 pitch low byte to 0x{:02X}", value);
                self.voice0_pitch = (self.voice0_pitch & 0xFF00) | (value as u16);
            }
            0x2D => {
                println!("Setting Voice 0 pitch high byte to 0x{:02X}", value);
                self.voice0_pitch = (self.voice0_pitch & 0x00FF) | ((value as u16) << 8);
            }
            0x4C => {
                println!("Setting Voice 0 volume left to 0x{:02X}", value);
                self.voice0_volume_left = value;
            }
            0x5C => {
                println!("Setting Voice 0 volume right to 0x{:02X}", value);
                self.voice0_volume_right = value;
            }
            _ => { /* no special action */ }
        }
    }

    pub fn read_selected_register(&self) -> u8 {
        let value = self.regs[self.selected as usize];
        println!("DSP: Read 0x{:02X} from register 0x{:02X}", value, self.selected);
        value
    }

    pub fn step(&mut self) {
        let keyon = self.regs[0x4C]; // KEYON register
        if keyon & 0x01 != 0 { // voice 0
            let pitch_low = self.regs[0x02]; // V0_PITCHL
            let pitch_high = self.regs[0x03]; // V0_PITCHH
            let pitch = ((pitch_high as u16) << 8) | (pitch_low as u16);
    
            let vol_left = self.regs[0x0C]; // V0_VOL_LEFT
            let vol_right = self.regs[0x1C]; // V0_VOL_RIGHT
    
            println!(
                "DSP: Voice 0 playing. Pitch: {}, Volume L: {}, Volume R: {}",
                pitch, vol_left, vol_right
            );
        }
    }
}

use apu::dsp::Dsp;
use apu::Memory;
use std::fs::File;
use std::io::Write;

fn main() {
    let mut dsp = Dsp::new();
    let mut mem = Memory::new();

    // --- Step 1: Fill memory with a simple waveform (square wave) ---
    let sample_start = 0x1000;
    let sample_end   = 0x1080; // 128 samples

    for i in 0..(sample_end - sample_start) {
        // Alternating 127 and -128 for square wave
        // 128 as unsigned = 0x80 = -128 as signed
        let val = if i % 2 == 0 { 127u8 } else { 128u8 };
        mem.write8(sample_start + i, val);
    }

    // --- Step 2: Configure the voice ---
    {
        let v = &mut dsp.voices[0];
        v.key_on        = true;
        v.sample_start  = sample_start;
        v.sample_end    = sample_end;
        v.current_addr  = sample_start;
        v.pitch         = 0x100; // increment = 1 sample per tick
        v.left_vol      = 80;
        v.right_vol     = 80;

        // ADSR setup (example values)
        v.adsr.adsr_mode     = true;
        v.adsr.attack_rate   = 10;   // fast attack
        v.adsr.decay_rate    = 5;
        v.adsr.sustain_level = 4;    // mid sustain
        v.adsr.release_rate  = 8;
    }

    // --- Step 3: Generate audio ---
    let mut audio_buffer: Vec<i16> = Vec::new();
    let sample_rate = 44100;
    let num_samples = sample_rate * 3; // 3 seconds

    for _ in 0..num_samples {
        dsp.step(&mem); // pitch stepping + sample fetch

        // render_audio expects a *slice of tuples*
       let (left, _right) = dsp.render_audio_single();
        audio_buffer.push(left);
    }

    // --- Step 4: Save to raw PCM file ---
    let mut file = File::create("square_adsr.raw").unwrap();
    for sample in audio_buffer {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }

    println!("Raw audio written to square_adsr.raw");
}

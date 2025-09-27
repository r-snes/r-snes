use apu::dsp::Dsp;
use apu::Memory;
use std::fs::File;
use std::io::Write;

fn main() {
    let mut dsp = Dsp::new();
    let mut mem = Memory::new();

    // --- Step 1: Fill memory with a simple waveform (square wave) ---
    let sample_start = 0x1000;
    let sample_end = 0x1080; // 128 samples
    for i in 0..(sample_end - sample_start) {
        // Alternating 127 and -128 for square wave
        let val = if i % 2 == 0 { 127u8 } else { 128u8 };
        mem.write8(sample_start + i, val);
    }

    // --- Step 2: Configure the voice ---
    dsp.voices[0].key_on = true;
    dsp.voices[0].sample_start = sample_start;
    dsp.voices[0].sample_end = sample_end;
    dsp.voices[0].current_addr = sample_start;
    dsp.voices[0].pitch = 0x100; // integer increment = 1
    dsp.voices[0].left_vol = 50;
    dsp.voices[0].right_vol = 50;

    // --- Step 3: Generate a short buffer ---
    let mut audio_buffer = vec![];
    let num_samples = 44100; // 1 second at 44.1kHz
    for _ in 0..num_samples {
        dsp.step(&mem);
        let mix = dsp.render_audio(1);
        audio_buffer.push(mix[0].0); // left channel only for simplicity
    }

    // --- Step 4: Save to raw PCM file (16-bit signed little-endian) ---
    let mut file = File::create("square_wave.raw").unwrap();
    for sample in audio_buffer {
        let bytes = sample.to_le_bytes();
        file.write_all(&bytes).unwrap();
    }

    println!("Raw audio written to square_wave.raw");
}

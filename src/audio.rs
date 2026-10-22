use std::time::{Duration, Instant};

use crate::{gui::Gui, rsnes::RSnesEmu};

/// Audio output of the running game.
pub struct GameAudio {
    /// Whether the audio device has failed and been disabled.
    failed: bool,
    /// Playback starts paused and only begins once TARGET_QUEUED_FRAMES are
    /// queued, so the device never starts from an empty queue.
    playing: bool,
    /// Whether the emulation ran last frame.
    was_running: bool,
}

impl GameAudio {
    /// Audio frames (32 kHz stereo pairs) to keep queued on the device:
    /// ~64 ms, about four video frames. Enough cushion that one slow frame
    /// never empties the queue, small enough to keep input-to-sound
    /// latency low.
    const TARGET_QUEUED_FRAMES: u32 = 2_048;
    /// The DSP's native output rate; queued frames / this = seconds queued.
    const AUDIO_RATE_HZ: f64 = 32_000.0;
    /// Longest single pacing sleep, so input and the window stay responsive
    /// even if the queue somehow grows far past the target.
    const MAX_PACING_SLEEP: Duration = Duration::from_millis(50);

    pub fn new() -> Self {
        Self {
            failed: false,
            playing: false,
            was_running: true,
        }
    }

    /// Pauses or resumes the device when the emulation stops or starts running.
    pub fn set_running(&mut self, gui: &Gui, running: bool) {
        if running != self.was_running && self.playing && !self.failed {
            if running {
                gui.audio_play();
            } else {
                gui.audio_pause();
            }
        }
        self.was_running = running;
    }

    /// Drain whatever audio the real hardware produced this frame and
    /// queue it straight through.
    pub fn queue_frame(&mut self, gui: &Gui, emu: &mut RSnesEmu) {
        if self.failed {
            return;
        }

        let samples = emu.core_mut().apu.drain_samples();
        if !samples.is_empty()
            && let Err(e) = gui.audio_queue_samples(&samples)
        {
            eprintln!("audio output disabled: {e}");
            gui.audio_stop();
            self.failed = true;
            return;
        }

        // Start playback once the cushion is built. If the queue ever runs
        // dry anyway (a long stall), pause and rebuild the cushion rather
        // than letting SDL alternate between sound and silence.
        let queued = gui.audio_buffered_frames();
        if !self.playing && queued >= Self::TARGET_QUEUED_FRAMES {
            gui.audio_play();
            self.playing = true;
        } else if self.playing && queued == 0 {
            gui.audio_stop();
            self.playing = false;
        }
    }

    /// Restarts the device after the console is reset.
    pub fn reset(&mut self, gui: &Gui) {
        gui.audio_stop();
        gui.audio_play();
        self.failed = false;
    }

    /// Pace on the audio queue: the sound device consumes exactly 32 kHz,
    /// so letting it set the speed keeps emulation in step with real
    /// time and absorbs the 60.0988 Hz (SNES) vs 60 Hz mismatch. Sleep
    /// only for the audio queued beyond the target; at or below it, go
    /// straight to the next frame. Without audio, or while paused, fall back to the fixed frame deadline.
    pub fn pacing_sleep(&self, gui: &Gui, running: bool, frame_start: Instant) -> Duration {
        if self.failed || !running {
            let deadline = frame_start + Duration::from_secs_f64(Gui::FRAME_DURATION);
            deadline.saturating_duration_since(Instant::now())
        } else if !self.playing {
            Duration::ZERO // still building the cushion: run flat out
        } else {
            let queued = gui.audio_buffered_frames();
            let excess = queued.saturating_sub(Self::TARGET_QUEUED_FRAMES);
            Duration::from_secs_f64(excess as f64 / Self::AUDIO_RATE_HZ).min(Self::MAX_PACING_SLEEP)
        }
    }

    // Stop the audio device.
    pub fn stop(&self, gui: &Gui) {
        if !self.failed {
            // Pause and discard anything still queued, whether or not
            // playback had started yet.
            gui.audio_stop();
        }
    }
}

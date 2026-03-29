//! Procedural audio playback — spatial sound synthesis.
//! Native: rodio/cpal. WASM: Web Audio API via web-sys.
//! All sounds generated mathematically (no audio files).

use std::f32::consts::{FRAC_PI_4, TAU};

const SAMPLE_RATE: u32 = 44100;

// ============================================================
// Platform-specific output backend
// ============================================================

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::*;
    use rodio::Source;

    pub struct AudioOutput {
        _stream: rodio::OutputStream,
        handle: rodio::OutputStreamHandle,
    }

    impl AudioOutput {
        pub fn new() -> Option<Self> {
            match rodio::OutputStream::try_default() {
                Ok((stream, handle)) => {
                    log::info!("Audio output initialized (native)");
                    Some(AudioOutput {
                        _stream: stream,
                        handle,
                    })
                }
                Err(e) => {
                    log::warn!("Audio init failed: {e}");
                    None
                }
            }
        }

        pub fn play_stereo_buffer(&self, left: &[f32], right: &[f32]) {
            let len = left.len().min(right.len());
            let mut interleaved = Vec::with_capacity(len * 2);
            for i in 0..len {
                interleaved.push(left[i]);
                interleaved.push(right[i]);
            }
            let source = StereoBuffer {
                data: interleaved,
                pos: 0,
            };
            let _ = self.handle.play_raw(source);
        }
    }

    struct StereoBuffer {
        data: Vec<f32>,
        pos: usize,
    }

    impl Iterator for StereoBuffer {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            if self.pos < self.data.len() {
                let s = self.data[self.pos];
                self.pos += 1;
                Some(s)
            } else {
                None
            }
        }
    }

    impl Source for StereoBuffer {
        fn current_frame_len(&self) -> Option<usize> {
            Some(self.data.len() - self.pos)
        }
        fn channels(&self) -> u16 {
            2
        }
        fn sample_rate(&self) -> u32 {
            SAMPLE_RATE
        }
        fn total_duration(&self) -> Option<std::time::Duration> {
            Some(std::time::Duration::from_secs_f64(
                (self.data.len() / 2) as f64 / SAMPLE_RATE as f64,
            ))
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::*;
    use wasm_bindgen::JsValue;
    use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext};

    pub struct AudioOutput {
        ctx: AudioContext,
    }

    impl AudioOutput {
        pub fn new() -> Option<Self> {
            match AudioContext::new() {
                Ok(ctx) => {
                    log::info!("Audio output initialized (WebAudio)");
                    Some(AudioOutput { ctx })
                }
                Err(e) => {
                    log::warn!("WebAudio init failed: {:?}", e);
                    None
                }
            }
        }

        pub fn play_stereo_buffer(&self, left: &[f32], right: &[f32]) {
            let len = left.len().min(right.len());
            if len == 0 {
                return;
            }
            let sr = SAMPLE_RATE as f32;

            let buffer = match self.ctx.create_buffer(2, len as u32, sr) {
                Ok(b) => b,
                Err(_) => return,
            };

            // Copy channel data
            if buffer.copy_to_channel(left, 0).is_err() {
                return;
            }
            if buffer.copy_to_channel(right, 1).is_err() {
                return;
            }

            let source: AudioBufferSourceNode = match self.ctx.create_buffer_source() {
                Ok(s) => s,
                Err(_) => return,
            };
            source.set_buffer(Some(&buffer));
            if source
                .connect_with_audio_node(&self.ctx.destination())
                .is_err()
            {
                return;
            }
            let _ = source.start();
        }
    }
}

pub use backend::AudioOutput;

// ============================================================
// Shared: synthesis + spatialization
// ============================================================

impl AudioOutput {
    /// Short UI click sound for button hover/press.
    pub fn play_click(&self) {
        // Very short high-frequency tick: 3ms noise burst
        let num_samples = (0.003 * SAMPLE_RATE as f32) as usize;
        let mut rng = Xorshift32::new(777);
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / num_samples as f32;
            let env = 1.0 - t; // linear decay
            let noise = rng.next_f32() * 2.0 - 1.0;
            // High-pass for a sharp tick
            samples.push(noise * env * 0.15);
        }
        self.play_stereo_buffer(&samples, &samples);
    }

    /// Play a test tone to verify audio output works.
    pub fn test_beep(&self) {
        let samples = render_sine(440.0, 0.5);
        let gain = 0.3;
        let left: Vec<f32> = samples.iter().map(|&s| s * gain).collect();
        let right = left.clone();
        self.play_stereo_buffer(&left, &right);
        log::info!("Audio test beep sent");
    }

    /// Play a spatialized procedural sound.
    pub fn play(
        &self,
        x: f32,
        y: f32,
        amplitude: f32,
        frequency: f32,
        pattern: u32,
        duration: f32,
        listener_x: f32,
        listener_y: f32,
    ) {
        let (left_gain, right_gain) = spatialize(listener_x, listener_y, x, y, amplitude);
        if left_gain < 0.001 && right_gain < 0.001 {
            return;
        }

        let mono = match pattern {
            0 => render_impulse(amplitude, duration),
            1 => render_sine(frequency, duration),
            2 => render_noise(frequency, duration),
            _ => return,
        };

        let left: Vec<f32> = mono.iter().map(|&s| s * left_gain).collect();
        let right: Vec<f32> = mono.iter().map(|&s| s * right_gain).collect();
        self.play_stereo_buffer(&left, &right);
    }
}

// --- Spatialization ---

fn spatialize(lx: f32, ly: f32, sx: f32, sy: f32, amplitude: f32) -> (f32, f32) {
    let dx = sx - lx;
    let dy = sy - ly;
    let dist = (dx * dx + dy * dy).sqrt().max(0.5);
    if dist > 60.0 {
        return (0.0, 0.0);
    }

    let gain = (amplitude / (1.0 + dist * dist * 0.08)).min(1.0) * 0.25;
    let pan = (dx / dist).clamp(-1.0, 1.0);
    let angle = (pan + 1.0) * FRAC_PI_4;
    (gain * angle.cos(), gain * angle.sin())
}

// --- Pre-rendered synthesis (shared across platforms) ---

fn render_impulse(amplitude: f32, duration: f32) -> Vec<f32> {
    let num_samples = (duration * SAMPLE_RATE as f32) as usize;
    let mut buffer = Vec::with_capacity(num_samples);
    let mut rng = Xorshift32::new(42);

    let db = 80.0 + (amplitude.max(0.001).log10() * 40.0);
    let decay_rate = 8.0 / duration;

    let mut prev_in = 0.0f32;
    let mut prev_out = 0.0f32;

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let envelope = (-t * decay_rate).exp();
        let noise = rng.next_f32() * 2.0 - 1.0;

        let filtered = if db > 115.0 {
            let alpha = 300.0 * TAU / (SAMPLE_RATE as f32 + 300.0 * TAU);
            prev_out = alpha * noise + (1.0 - alpha) * prev_out;
            let sub_bass = (t * 60.0 * TAU).sin() * 0.4;
            prev_out + sub_bass
        } else if db > 90.0 {
            let alpha = SAMPLE_RATE as f32 / (SAMPLE_RATE as f32 + 2000.0 * TAU);
            let hp = alpha * (prev_out + noise - prev_in);
            prev_in = noise;
            prev_out = hp;
            hp
        } else {
            let alpha_lp = 800.0 * TAU / (SAMPLE_RATE as f32 + 800.0 * TAU);
            prev_out = alpha_lp * noise + (1.0 - alpha_lp) * prev_out;
            let alpha_hp = SAMPLE_RATE as f32 / (SAMPLE_RATE as f32 + 200.0 * TAU);
            let hp = alpha_hp * (prev_out - prev_in);
            prev_in = prev_out;
            hp
        };

        buffer.push(filtered * envelope);
    }
    buffer
}

fn render_sine(frequency: f32, duration: f32) -> Vec<f32> {
    let total_samples = (duration * SAMPLE_RATE as f32) as usize;
    let mut buffer = Vec::with_capacity(total_samples);
    let mut phase: f64 = 0.0;

    for i in 0..total_samples {
        let t = i as f64 / SAMPLE_RATE as f64;
        let frac = i as f32 / total_samples as f32;

        let vibrato = (t * 0.3 * TAU as f64).sin() * 0.05 * frequency as f64;
        let freq = frequency as f64 + vibrato;
        phase += freq * TAU as f64 / SAMPLE_RATE as f64;
        let sample = phase.sin() as f32;

        let env = if frac < 0.15 {
            let x = frac / 0.15;
            x * x * (3.0 - 2.0 * x)
        } else if frac > 0.75 {
            let x = (1.0 - frac) / 0.25;
            x * x * (3.0 - 2.0 * x)
        } else {
            1.0
        };

        buffer.push(sample * env);
    }
    buffer
}

fn render_noise(frequency: f32, duration: f32) -> Vec<f32> {
    let total_samples = (duration * SAMPLE_RATE as f32) as usize;
    let mut buffer = Vec::with_capacity(total_samples);
    let mut rng = Xorshift32::new(0xDEAD_BEEF);
    let frequency = frequency.max(100.0);

    let mut lp_state = 0.0f32;
    let mut hp_prev_in = 0.0f32;
    let mut hp_prev_out = 0.0f32;

    for i in 0..total_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let frac = i as f32 / total_samples as f32;

        let noise = rng.next_f32() * 2.0 - 1.0;

        let lp_freq = frequency * 1.5;
        let hp_freq = frequency * 0.5;
        let alpha_lp = lp_freq * TAU / (SAMPLE_RATE as f32 + lp_freq * TAU);
        lp_state = alpha_lp * noise + (1.0 - alpha_lp) * lp_state;

        let alpha_hp = SAMPLE_RATE as f32 / (SAMPLE_RATE as f32 + hp_freq * TAU);
        let hp = alpha_hp * (hp_prev_out + lp_state - hp_prev_in);
        hp_prev_in = lp_state;
        hp_prev_out = hp;

        let click_rate = 15.0;
        let click_env = (t * click_rate * std::f32::consts::PI).sin().abs().powi(4);
        let fade = 1.0 - frac;

        buffer.push(hp * click_env * fade * 1.5);
    }
    buffer
}

// --- Minimal PRNG ---

struct Xorshift32 {
    state: u32,
}

impl Xorshift32 {
    fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0x7FFFFF) as f32 / 0x7FFFFF as f32
    }
}

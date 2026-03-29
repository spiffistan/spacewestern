//! Procedural audio playback — spatial sound synthesis using rodio.
//! Generates all sounds mathematically (no audio files).

use rodio::Source;
use std::f32::consts::{FRAC_PI_4, TAU};

const SAMPLE_RATE: u32 = 44100;

/// Audio output device. Holds the rodio stream (must stay alive).
pub struct AudioOutput {
    _stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
}

impl AudioOutput {
    pub fn new() -> Option<Self> {
        match rodio::OutputStream::try_default() {
            Ok((stream, handle)) => {
                log::info!("Audio output initialized successfully");
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

    /// Play a test tone to verify audio output works.
    pub fn test_beep(&self) {
        let src = SineSource::new(440.0, 0.5);
        let spatial = SpatialStereo::new(src, 0.3, 0.3);
        let _ = self.handle.play_raw(spatial.convert_samples());
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

        match pattern {
            0 => {
                let src = ImpulseSource::new(amplitude, duration);
                let spatial = SpatialStereo::new(src, left_gain, right_gain);
                let _ = self.handle.play_raw(spatial.convert_samples());
            }
            1 => {
                let src = SineSource::new(frequency, duration);
                let spatial = SpatialStereo::new(src, left_gain, right_gain);
                let _ = self.handle.play_raw(spatial.convert_samples());
            }
            2 => {
                let src = NoiseSource::new(frequency, duration);
                let spatial = SpatialStereo::new(src, left_gain, right_gain);
                let _ = self.handle.play_raw(spatial.convert_samples());
            }
            _ => {}
        }
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

    // Inverse-distance falloff, normalize amplitude (80dB = 1.0 in game scale)
    let gain = (amplitude / (1.0 + dist * dist * 0.08)).min(1.0) * 0.25;

    // Stereo panning: constant-power pan law
    let pan = (dx / dist).clamp(-1.0, 1.0);
    let angle = (pan + 1.0) * FRAC_PI_4; // 0..PI/2
    (gain * angle.cos(), gain * angle.sin())
}

// --- Spatial stereo wrapper ---

struct SpatialStereo<S> {
    inner: S,
    left_gain: f32,
    right_gain: f32,
    is_left: bool,
    current_sample: f32,
}

impl<S> SpatialStereo<S> {
    fn new(inner: S, left_gain: f32, right_gain: f32) -> Self {
        Self {
            inner,
            left_gain,
            right_gain,
            is_left: true,
            current_sample: 0.0,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for SpatialStereo<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.is_left {
            self.current_sample = self.inner.next()?;
            self.is_left = false;
            Some(self.current_sample * self.left_gain)
        } else {
            self.is_left = true;
            Some(self.current_sample * self.right_gain)
        }
    }
}

impl<S: Source<Item = f32>> Source for SpatialStereo<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len().map(|n| n * 2)
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

// --- Impulse source (gunshot, explosion, door slam) ---

struct ImpulseSource {
    buffer: Vec<f32>,
    pos: usize,
}

impl ImpulseSource {
    fn new(amplitude: f32, duration: f32) -> Self {
        let num_samples = (duration * SAMPLE_RATE as f32) as usize;
        let mut buffer = Vec::with_capacity(num_samples);
        let mut rng = Xorshift32::new(42);

        // Determine character from amplitude
        let db = 80.0 + (amplitude.max(0.001).log10() * 40.0);
        let decay_rate = 8.0 / duration;

        // Filter state
        let mut prev_in = 0.0f32;
        let mut prev_out = 0.0f32;

        for i in 0..num_samples {
            let t = i as f32 / SAMPLE_RATE as f32;
            let envelope = (-t * decay_rate).exp();
            let noise = rng.next_f32() * 2.0 - 1.0;

            let filtered = if db > 115.0 {
                // Explosion: low-pass 300Hz + sub-bass
                let alpha = 300.0 * TAU / (SAMPLE_RATE as f32 + 300.0 * TAU);
                prev_out = alpha * noise + (1.0 - alpha) * prev_out;
                let sub_bass = (t * 60.0 * TAU).sin() * 0.4;
                prev_out + sub_bass
            } else if db > 90.0 {
                // Gunshot: high-pass 2kHz
                let alpha = SAMPLE_RATE as f32 / (SAMPLE_RATE as f32 + 2000.0 * TAU);
                let hp = alpha * (prev_out + noise - prev_in);
                prev_in = noise;
                prev_out = hp;
                hp
            } else {
                // Door slam: band-pass 200-800Hz (LP then HP)
                let alpha_lp = 800.0 * TAU / (SAMPLE_RATE as f32 + 800.0 * TAU);
                prev_out = alpha_lp * noise + (1.0 - alpha_lp) * prev_out;
                let alpha_hp = SAMPLE_RATE as f32 / (SAMPLE_RATE as f32 + 200.0 * TAU);
                let hp = alpha_hp * (prev_out - prev_in);
                prev_in = prev_out;
                hp
            };

            buffer.push(filtered * envelope);
        }

        ImpulseSource { buffer, pos: 0 }
    }
}

impl Iterator for ImpulseSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.pos < self.buffer.len() {
            let s = self.buffer[self.pos];
            self.pos += 1;
            Some(s)
        } else {
            None
        }
    }
}

impl Source for ImpulseSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buffer.len() - self.pos)
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs_f32(
            self.buffer.len() as f32 / SAMPLE_RATE as f32,
        ))
    }
}

// --- Sine source (hollowcall tone, alarm) ---

struct SineSource {
    frequency: f32,
    phase: f64,
    sample_idx: u64,
    total_samples: u64,
}

impl SineSource {
    fn new(frequency: f32, duration: f32) -> Self {
        Self {
            frequency,
            phase: 0.0,
            sample_idx: 0,
            total_samples: (duration * SAMPLE_RATE as f32) as u64,
        }
    }
}

impl Iterator for SineSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.sample_idx >= self.total_samples {
            return None;
        }
        let t = self.sample_idx as f64 / SAMPLE_RATE as f64;
        let frac = self.sample_idx as f32 / self.total_samples as f32;

        // Vibrato: slow pitch drift
        let vibrato = (t * 0.3 * TAU as f64).sin() * 0.05 * self.frequency as f64;
        let freq = self.frequency as f64 + vibrato;

        self.phase += freq * TAU as f64 / SAMPLE_RATE as f64;
        let sample = self.phase.sin() as f32;

        // Envelope: smoothstep fade-in (first 15%) and fade-out (last 25%)
        let env = if frac < 0.15 {
            let x = frac / 0.15;
            x * x * (3.0 - 2.0 * x)
        } else if frac > 0.75 {
            let x = (1.0 - frac) / 0.25;
            x * x * (3.0 - 2.0 * x)
        } else {
            1.0
        };

        self.sample_idx += 1;
        Some(sample * env)
    }
}

impl Source for SineSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some((self.total_samples - self.sample_idx) as usize)
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs_f64(
            self.total_samples as f64 / SAMPLE_RATE as f64,
        ))
    }
}

// --- Noise source (duskweaver chittering) ---

struct NoiseSource {
    rng: Xorshift32,
    frequency: f32,
    sample_idx: u64,
    total_samples: u64,
    // Band-pass filter state
    lp_state: f32,
    hp_prev_in: f32,
    hp_prev_out: f32,
}

impl NoiseSource {
    fn new(frequency: f32, duration: f32) -> Self {
        Self {
            rng: Xorshift32::new(0xDEAD_BEEF),
            frequency: frequency.max(100.0),
            sample_idx: 0,
            total_samples: (duration * SAMPLE_RATE as f32) as u64,
            lp_state: 0.0,
            hp_prev_in: 0.0,
            hp_prev_out: 0.0,
        }
    }
}

impl Iterator for NoiseSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.sample_idx >= self.total_samples {
            return None;
        }
        let t = self.sample_idx as f32 / SAMPLE_RATE as f32;
        let frac = self.sample_idx as f32 / self.total_samples as f32;

        // White noise
        let noise = self.rng.next_f32() * 2.0 - 1.0;

        // Band-pass: low-pass at freq*1.5, then high-pass at freq*0.5
        let lp_freq = self.frequency * 1.5;
        let hp_freq = self.frequency * 0.5;
        let alpha_lp = lp_freq * TAU / (SAMPLE_RATE as f32 + lp_freq * TAU);
        self.lp_state = alpha_lp * noise + (1.0 - alpha_lp) * self.lp_state;

        let alpha_hp = SAMPLE_RATE as f32 / (SAMPLE_RATE as f32 + hp_freq * TAU);
        let hp = alpha_hp * (self.hp_prev_out + self.lp_state - self.hp_prev_in);
        self.hp_prev_in = self.lp_state;
        self.hp_prev_out = hp;

        // Rhythmic clicking: amplitude modulation
        let click_rate = 15.0;
        let click_env = (t * click_rate * std::f32::consts::PI).sin().abs().powi(4);

        // Overall fade-out
        let fade = 1.0 - frac;

        self.sample_idx += 1;
        Some(hp * click_env * fade * 1.5)
    }
}

impl Source for NoiseSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some((self.total_samples - self.sample_idx) as usize)
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs_f64(
            self.total_samples as f64 / SAMPLE_RATE as f64,
        ))
    }
}

// --- Minimal PRNG (no dependencies) ---

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

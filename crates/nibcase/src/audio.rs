//! Streaming stereo resampling with a 64-tap, 1024-phase Blackman-Harris windowed sinc.
//! Small rate changes alter the phase increment, preserving history across video frames.
const TAPS: usize = 64;
const PHASES: usize = 1024;

pub struct Resampler {
    table: Vec<[f32; TAPS]>,
    samples: Vec<[f32; 2]>,
    position: u64,
    output: Vec<f32>,
}
impl Resampler {
    pub fn new(input_rate: f64) -> Self {
        let cutoff = 0.90f64.min(48000.0 / input_rate * 0.90);
        let mut table = Vec::with_capacity(PHASES);
        for phase in 0..PHASES {
            let fraction = phase as f64 / PHASES as f64;
            let mut coefficients = [0.0; TAPS];
            let mut sum = 0.0;
            for (tap, v) in coefficients.iter_mut().enumerate() {
                let x = tap as f64 - (TAPS / 2 - 1) as f64 - fraction;
                let n = tap as f64 / (TAPS - 1) as f64;
                let a = std::f64::consts::TAU * n;
                let window = 0.35875 - 0.48829 * a.cos() + 0.14128 * (2.0 * a).cos()
                    - 0.01168 * (3.0 * a).cos();
                let sinc = if x.abs() < 1e-12 {
                    cutoff
                } else {
                    (std::f64::consts::PI * cutoff * x).sin() / (std::f64::consts::PI * x)
                };
                *v = (sinc * window) as f32;
                sum += f64::from(*v);
            }
            for v in &mut coefficients {
                *v /= sum as f32;
            }
            table.push(coefficients);
        }
        Self {
            table,
            samples: vec![[0.0; 2]; TAPS / 2],
            position: ((TAPS / 2 - 1) as u64) << 32,
            output: Vec::new(),
        }
    }
    pub fn clear(&mut self) {
        self.samples.clear();
        self.samples.resize(TAPS / 2, [0.0; 2]);
        self.position = ((TAPS / 2 - 1) as u64) << 32;
        self.output.clear();
    }
    pub fn process(&mut self, input: &[i16], step: f64) -> &[f32] {
        self.samples.extend(
            input
                .chunks_exact(2)
                .map(|v| [f32::from(v[0]) / 32768.0, f32::from(v[1]) / 32768.0]),
        );
        self.output.clear();
        if !(0.01..=32.0).contains(&step) {
            return &self.output;
        }
        let increment = (step * 4294967296.0).round() as u64;
        while (self.position >> 32) as usize + TAPS / 2 < self.samples.len() {
            let center = (self.position >> 32) as usize;
            let phase = (((self.position & 0xffffffff) * PHASES as u64) >> 32) as usize;
            let start = center - (TAPS / 2 - 1);
            let mut out = [0.0; 2];
            for (sample, weight) in self.samples[start..start + TAPS]
                .iter()
                .zip(&self.table[phase])
            {
                out[0] += sample[0] * weight;
                out[1] += sample[1] * weight;
            }
            self.output.extend(out);
            self.position += increment;
        }
        let consumed = ((self.position >> 32) as usize)
            .saturating_sub(TAPS / 2 - 1)
            .min(self.samples.len());
        if consumed > 0 {
            self.samples.drain(..consumed);
            self.position -= (consumed as u64) << 32;
        }
        &self.output
    }
}
/// Source samples per output sample. Smooth timing follows the display; PAL retains 50 Hz.
pub fn step(input_rate: f64, core_fps: f64, play_fps: f64, queued_frames: f64, target: f64) -> f64 {
    let correction = ((queued_frames - target) * 0.000002).clamp(-0.005, 0.005);
    input_rate / 48000.0 * play_fps / core_fps * (1.0 + correction)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streamed_chunks_equal_one_buffer_and_rates_dont_drift() {
        let source: Vec<i16> = (0..32040)
            .flat_map(|i| {
                let s =
                    ((i as f64 * std::f64::consts::TAU * 1000.0 / 32040.0).sin() * 24000.0) as i16;
                [s, -s]
            })
            .collect();
        let mut full = Resampler::new(32040.0);
        let a = full.process(&source, 32040.0 / 48000.0).to_vec();
        let mut chunks = Resampler::new(32040.0);
        let mut b = Vec::new();
        for chunk in source.chunks(1068) {
            b.extend_from_slice(chunks.process(chunk, 32040.0 / 48000.0));
        }
        assert!(
            a == b,
            "chunk boundaries must not change the resampling phase"
        );
        assert!((a.len() as i64 / 2 - 48000).abs() < 64);
        assert!(a.chunks_exact(2).all(|v| (v[0] + v[1]).abs() < 1e-7));
    }
    #[test]
    fn dc_gain_queue_correction_and_reset() {
        let mut r = Resampler::new(32040.5);
        let dc = vec![12000; 6400];
        let out = r.process(&dc, 32040.5 / 48000.0);
        assert!(out[256..]
            .iter()
            .all(|v| (*v - 12000.0 / 32768.0).abs() < 0.0001));
        assert!(
            step(32040.5, 60.0988, 59.94, 2000.0, 1024.0)
                > step(32040.5, 60.0988, 59.94, 1000.0, 1024.0)
        );
        r.clear();
        assert!(r.process(&[], 1.0).is_empty());
    }
    #[test]
    fn downsampling_rejects_above_nyquist() {
        let mut r = Resampler::new(96000.0);
        let src: Vec<i16> = (0..9600)
            .flat_map(|i| {
                let n =
                    ((std::f64::consts::TAU * 32000.0 * i as f64 / 96000.0).sin() * 30000.0) as i16;
                [n, n]
            })
            .collect();
        let out = r.process(&src, 2.0);
        let rms = (out[256..].iter().map(|n| n * n).sum::<f32>() / (out.len() - 256) as f32).sqrt();
        assert!(rms < 0.0001, "alias RMS {rms}");
    }
}

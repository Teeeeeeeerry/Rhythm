/// Simple linear-interpolation resampler with channel mapping.
///
/// symphonia-core 0.5 does not ship a resampler, so this converts decoded
/// interleaved f32 PCM at the stream's native sample rate/channel count to the
/// audio device's rate/channels. Linear interpolation is low-cost and fine for
/// the small rate ratios found in practice (44.1k ↔ 48k).
///
/// Channel mapping: output channel `i` uses input plane `i` when available;
/// mono input is duplicated to all output channels; missing planes are muted.
/// (Plane order follows symphonia's `Channels` bit order, so plane 0/1 are the
/// front left/right channels on standard layouts.)
pub struct Resampler {
    in_rate: u32,
    in_channels: u16,
    out_rate: u32,
    out_channels: u16,
    /// Source position (in input frames) of the next output frame.
    src_pos: f64,
}

impl Resampler {
    /// Create a resampler converting `(in_rate, in_channels)` interleaved f32
    /// to `(out_rate, out_channels)` interleaved f32.
    pub fn new(in_rate: u32, in_channels: u16, out_rate: u32, out_channels: u16) -> Self {
        Resampler {
            in_rate: in_rate.max(1),
            in_channels: in_channels.max(1),
            out_rate: out_rate.max(1),
            out_channels: out_channels.max(1),
            src_pos: 0.0,
        }
    }

    /// True when no conversion is needed (same rate and channel count).
    #[allow(dead_code)] // exercised by tests
    pub fn is_identity(&self) -> bool {
        self.in_rate == self.out_rate && self.in_channels == self.out_channels
    }

    /// Reset the resampler's phase (e.g. after a seek).
    pub fn reset(&mut self) {
        self.src_pos = 0.0;
    }

    /// Resample as many complete output frames as possible from `input`
    /// (interleaved `in_channels`), writing interleaved `out_channels` frames
    /// into `output`. Returns the number of output frames written. State is
    /// preserved between calls so chunk boundaries are seamless.
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> usize {
        if input.is_empty() || output.is_empty() {
            return 0;
        }
        let in_frames = input.len() / self.in_channels as usize;
        if in_frames == 0 {
            return 0;
        }
        // Source position of the next output frame, in input frames: output
        // frame `k` happens at time `k / out_rate` seconds, i.e. at input
        // frame `k * in_rate / out_rate`.
        let step = f64::from(self.in_rate) / f64::from(self.out_rate);
        let out_frames = output.len() / self.out_channels as usize;

        let mut written = 0usize;
        while written < out_frames {
            let src = self.src_pos;
            if src >= in_frames as f64 {
                break;
            }
            let idx = src.floor() as usize;
            let frac = (src - idx as f64) as f32;
            let next = (idx + 1).min(in_frames - 1);

            for ch in 0..self.out_channels as usize {
                let s0 = Self::sample_at(input, idx, ch, self.in_channels);
                let s1 = Self::sample_at(input, next, ch, self.in_channels);
                output[written * self.out_channels as usize + ch] = s0 + (s1 - s0) * frac;
            }
            written += 1;
            self.src_pos += step;
        }

        written
    }

    /// Get the sample for input frame `idx`, channel `ch`. Mono input is
    /// duplicated to every channel; channels beyond the input layout are
    /// muted. `idx` must be in `[0, in_frames)`.
    fn sample_at(input: &[f32], idx: usize, ch: usize, in_channels: u16) -> f32 {
        if in_channels == 1 {
            return input[idx];
        }
        if ch >= in_channels as usize {
            return 0.0;
        }
        input[idx * in_channels as usize + ch]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_passthrough() {
        // Same rate/channels: process should copy frames 1:1.
        let mut r = Resampler::new(44100, 2, 44100, 2);
        assert!(r.is_identity());
        let input = vec![0.0f32, 1.0, 0.5, -0.5];
        let mut out = vec![0.0f32; 4];
        let n = r.process(&input, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out, input);
    }

    #[test]
    fn test_mono_to_stereo() {
        let mut r = Resampler::new(44100, 1, 44100, 2);
        let input = vec![0.25f32, -0.75];
        let mut out = vec![0.0f32; 4];
        let n = r.process(&input, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out, vec![0.25, 0.25, -0.75, -0.75]);
    }

    #[test]
    fn test_half_speed_downsample() {
        // 48k → 24k, ratio 0.5. Two input frames produce one output frame per
        // half-step; with 2 input frames only the first step completes.
        let mut r = Resampler::new(48000, 2, 24000, 2);
        let input = vec![0.0f32, 0.0, 1.0, 1.0];
        let mut out = vec![0.0f32; 4];
        let n = r.process(&input, &mut out);
        assert_eq!(n, 1);
        assert_eq!(out, vec![0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_interpolation() {
        // 48k → 96k, ratio 2.0: output frame 1 interpolates halfway between
        // input frames 0 and 1.
        let mut r = Resampler::new(48000, 1, 96000, 1);
        let input = vec![0.0f32, 1.0];
        let mut out = vec![0.0f32; 4];
        let n = r.process(&input, &mut out);
        assert_eq!(n, 4);
        assert_eq!(out[0], 0.0);
        assert!((out[1] - 0.5).abs() < 1e-6);
        assert_eq!(out[2], 1.0);
        assert_eq!(out[3], 1.0); // clamped to last input frame
    }
}

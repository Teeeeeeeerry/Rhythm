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
    /// Last input frame from the previous `process` call, one sample per
    /// input channel. Used for correct interpolation across block boundaries
    /// so the output doesn't repeat the last sample of every chunk.
    prev_tail: Vec<f32>,
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
            prev_tail: Vec::new(),
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
        self.prev_tail.clear();
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
            let next = if idx + 1 < in_frames {
                idx + 1
            } else {
                // At the last frame of this block: cross-block interpolation
                // uses the stored previous-block tail when available,
                // otherwise clamps to the current last frame.
                idx
            };

            for ch in 0..self.out_channels as usize {
                let s0 = Self::sample_at(input, idx, ch, self.in_channels);
                let s1 = if next == idx && !self.prev_tail.is_empty() {
                    // Use the tail from the previous block for proper
                    // cross-block interpolation.
                    Self::sample_at(&self.prev_tail, 0, ch, self.in_channels)
                } else {
                    Self::sample_at(input, next, ch, self.in_channels)
                };
                output[written * self.out_channels as usize + ch] = s0 + (s1 - s0) * frac;
            }
            written += 1;
            self.src_pos += step;
        }

        // Wrap the phase accumulator back into this block's coordinate system
        // so the *next* call starts from the correct position instead of
        // immediately hitting `src >= in_frames` and returning 0 (#28).
        self.src_pos -= in_frames as f64;
        if self.src_pos < 0.0 {
            self.src_pos = 0.0;
        }

        // Stash the last input frame for cross-block interpolation on the
        // next call. Without this every block boundary repeats the last
        // sample instead of interpolating toward the next block's first
        // sample, producing periodic subtle distortion.
        let tail_len = self.in_channels as usize;
        if input.len() >= tail_len {
            self.prev_tail
                .resize(tail_len, 0.0);
            let start = input.len() - tail_len;
            self.prev_tail.copy_from_slice(&input[start..]);
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

    /// Regression test for #28: after the first block `src_pos` was never
    /// wrapped back, so every subsequent call returned 0 frames — the song
    /// went silent and the progress bar raced to the end on empty buffers.
    #[test]
    fn multi_block_44k_to_48k_continuous_output() {
        let mut r = Resampler::new(44100, 2, 48000, 2);
        let mut total_written = 0usize;
        let blocks: Vec<Vec<f32>> = (0..5)
            .map(|b| {
                (0..1024 * 2)
                    .map(|i| (b * 1024 * 2 + i) as f32 / 1000.0)
                    .collect()
            })
            .collect();
        let block_frames = 1024;

        for block in &blocks {
            let out_frames =
                (block_frames as f64 * 48000.0 / 44100.0).ceil() as usize + 1;
            let mut out = vec![0.0f32; out_frames * 2];
            let n = r.process(block, &mut out);
            total_written += n;
        }

        // 5 blocks of 1024 frames at ratio 48/44.1 ≈ 1.0884 each.
        // Total output should be ~5572 frames; a bug returning 0 after the
        // first block would give only ~1115.
        let expected = (5.0 * block_frames as f64 * 48000.0 / 44100.0).ceil() as usize;
        let diff = (total_written as isize - expected as isize).unsigned_abs();
        assert!(
            diff <= 3,
            "total written {total_written} far from expected ~{expected} (diff {diff})"
        );
    }

    /// Identity resampler (same rate, same channels) must also work across
    /// multiple calls — the wrapping bug affected every ratio equally.
    #[test]
    fn multi_block_identity_continuous_output() {
        let mut r = Resampler::new(48000, 2, 48000, 2);
        assert!(r.is_identity());
        let mut total_written = 0usize;
        let block_frames = 1024;

        for b in 0..5 {
            let block: Vec<f32> = (0..block_frames * 2)
                .map(|i| (b * block_frames * 2 + i) as f32)
                .collect();
            let mut out = vec![0.0f32; block_frames * 2];
            let n = r.process(&block, &mut out);
            total_written += n;
            // Identity: every call should produce exactly block_frames output.
            assert_eq!(n, block_frames, "block {b}: expected {block_frames} frames, got {n}");
        }

        assert_eq!(total_written, 5 * 1024);
    }

    /// RM-07: after `reset` (the seek path) the phase is cleared and output
    /// matches a fresh instance fed the same input.
    #[test]
    fn reset_matches_fresh_instance() {
        let block: Vec<f32> = (0..2048).map(|i| i as f32 / 2048.0).collect();

        let mut warmed = Resampler::new(44100, 2, 48000, 2);
        warmed.process(&block, &mut vec![0.0f32; 4096]);
        warmed.reset();
        let mut warmed_out = vec![0.0f32; 4096];
        let n1 = warmed.process(&block, &mut warmed_out);

        let mut fresh = Resampler::new(44100, 2, 48000, 2);
        let mut fresh_out = vec![0.0f32; 4096];
        let n2 = fresh.process(&block, &mut fresh_out);

        assert_eq!(n1, n2);
        assert_eq!(warmed_out[..n1 * 2], fresh_out[..n1 * 2]);
    }

    /// RM-08: empty input or output produces zero frames.
    #[test]
    fn empty_input_or_output_yields_zero() {
        let mut r = Resampler::new(44100, 2, 48000, 2);
        let mut out = vec![0.0f32; 8];
        assert_eq!(r.process(&[], &mut out), 0);
        assert_eq!(r.process(&[0.1, 0.2], &mut []), 0);
    }

    /// RM-09: stereo → quad mutes the extra channels; quad → stereo keeps the
    /// first planes.
    #[test]
    fn channel_mapping_extra_and_missing_planes() {
        // stereo → quad: extra channels are silent.
        let mut up = Resampler::new(44100, 2, 44100, 4);
        let input = vec![0.25f32, -0.75, 1.0, -1.0];
        let mut out = vec![0.0f32; 8];
        let n = up.process(&input, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out, vec![0.25, -0.75, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0]);

        // quad → stereo: only the first two planes survive.
        let mut down = Resampler::new(44100, 4, 44100, 2);
        let input = vec![1.0f32, 2.0, 3.0, 4.0];
        let mut out = vec![0.0f32; 2];
        let n = down.process(&input, &mut out);
        assert_eq!(n, 1);
        assert_eq!(out, vec![1.0, 2.0]);
    }

    /// RM-10: an output buffer smaller than one frame writes nothing and does
    /// not consume the input — the next call still starts at the input head.
    #[test]
    fn short_output_buffer_consumes_nothing() {
        let mut r = Resampler::new(44100, 2, 48000, 2);
        let input: Vec<f32> = (0..2048).map(|i| i as f32 / 2048.0).collect();

        let mut tiny = vec![0.0f32; 1];
        assert_eq!(r.process(&input, &mut tiny), 0);

        let mut out = vec![0.0f32; 4096];
        let n = r.process(&input, &mut out);

        let mut fresh = Resampler::new(44100, 2, 48000, 2);
        let mut fresh_out = vec![0.0f32; 4096];
        let n2 = fresh.process(&input, &mut fresh_out);
        assert_eq!(n, n2);
        assert_eq!(out[..n * 2], fresh_out[..n * 2]);
    }

    /// RM-11: input shorter than one frame yields zero frames.
    #[test]
    fn short_input_yields_zero() {
        let mut r = Resampler::new(44100, 2, 48000, 2);
        let mut out = vec![0.0f32; 8];
        assert_eq!(r.process(&[0.5], &mut out), 0);
    }

    /// RM-12: extreme upsampling (8k → 192k, ×24) stays in bounds and
    /// produces no NaN.
    #[test]
    fn extreme_upsampling_no_nan() {
        let mut r = Resampler::new(8000, 1, 192000, 1);
        let frames = 128;
        let input: Vec<f32> = (0..frames).map(|i| i as f32).collect();
        let mut out = vec![0.0f32; frames * 24 + 2];
        let n = r.process(&input, &mut out);
        assert_eq!(n, frames * 24);
        assert!(out[..n].iter().all(|s| s.is_finite()), "NaN in output");
    }
}

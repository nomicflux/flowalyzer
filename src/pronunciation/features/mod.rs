const FRAME_LENGTH_SAMPLES_AT_16K: usize = 1024;
const HOP_SAMPLES_AT_16K: usize = 160; // 10ms at 16kHz

#[derive(Debug, Clone, Copy)]
pub struct FeatureConfig {
    pub frame_len_samples: usize,
    pub hop_samples: usize,
}

impl FeatureConfig {
    pub fn from_sample_rate(sample_rate: u32) -> Self {
        let frame_len_samples = scaled_samples(FRAME_LENGTH_SAMPLES_AT_16K, sample_rate);
        let hop_samples = scaled_samples(HOP_SAMPLES_AT_16K, sample_rate);
        Self {
            frame_len_samples,
            hop_samples,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceFeatures {
    pub energy: Vec<f32>,
    pub pitch: Vec<f32>,
    pub frame_starts: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct ChunkFeatures {
    pub energy: Vec<f32>,
    pub pitch: Vec<f32>,
    pub frame_starts: Vec<isize>,
}

#[derive(Debug, Clone, Copy)]
pub struct FeatureExtractor;

impl FeatureExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_reference(
        &self,
        samples: &[f32],
        sample_rate: u32,
        cfg: FeatureConfig,
    ) -> ReferenceFeatures {
        let mut starts = Vec::new();
        let mut energy = Vec::new();
        let mut pitch = Vec::new();
        let frame_len = cfg.frame_len_samples;
        let hop = cfg.hop_samples;
        let frame = &samples[0..frame_len];
        starts.push(0);
        energy.push(frame_energy(frame));
        pitch.push(frame_pitch(frame, sample_rate));

        // Loop for subsequent frames.
        // Range is 0..=(samples.len() - frame_len) to ensure we only take full frames.
        // If samples.len() < frame_len, this subtraction panics (natural panic).
        // But we already panicked above if that was the case.
        // We start at hop because 0 is already done.
        for start in (hop..=(samples.len() - frame_len)).step_by(hop) {
            let frame = &samples[start..start + frame_len];
            starts.push(start);
            energy.push(frame_energy(frame));
            pitch.push(frame_pitch(frame, sample_rate));
        }
        ReferenceFeatures {
            energy,
            pitch,
            frame_starts: starts,
        }
    }

    pub fn extract_chunk(
        &self,
        prev_tail: &[f32],
        chunk: &[f32],
        sample_rate: u32,
        cfg: FeatureConfig,
        phase_offset: usize,
    ) -> ChunkFeatures {
        let mut window = Vec::with_capacity(prev_tail.len() + chunk.len());
        window.extend_from_slice(prev_tail);
        window.extend_from_slice(chunk);

        let mut energy = Vec::new();
        let mut pitch = Vec::new();
        let mut frame_starts = Vec::new();
        let frame_len = cfg.frame_len_samples;
        let hop = cfg.hop_samples;
        let tail_len = prev_tail.len() as isize;

        // Calculate the first frame start index relative to the window that satisfies the global grid.
        // The window starts at `global_counter - tail_len`.
        // We want `(global_start + s) % hop == 0`.
        // We are given `phase_offset` which should be `(global_counter - tail_len) % hop`.
        // So `(phase_offset + s) % hop == 0` => `s = (hop - phase_offset) % hop`.
        let start_offset = (hop - phase_offset) % hop;

        // Ensure we don't iterate if window is too small (natural panic via range if we forced it,
        // but here we just want valid frames).
        // Actually, if window is too small for *any* frame, we return empty features?
        // The plan says "Invalid inputs fail where slices/divisions naturally panic".
        // But `extract_chunk` might validly return empty if the chunk is tiny and doesn't complete a frame.
        // So we iterate valid frames.

        if window.len() >= frame_len {
            for start in (start_offset..=(window.len() - frame_len)).step_by(hop) {
                let frame = &window[start..start + frame_len];
                let chunk_start = start as isize - (tail_len - hop as isize);
                if chunk_start >= 0 {
                    frame_starts.push(chunk_start);
                    energy.push(frame_energy(frame));
                    pitch.push(frame_pitch(frame, sample_rate));
                }
            }
        }
        ChunkFeatures {
            energy,
            pitch,
            frame_starts,
        }
    }
}

impl Default for FeatureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

fn scaled_samples(base_at_16k: usize, sample_rate: u32) -> usize {
    ((base_at_16k as u64) * sample_rate as u64 / 16_000) as usize
}

fn frame_energy(frame: &[f32]) -> f32 {
    frame.iter().map(|value| value * value).sum()
}

fn frame_pitch(frame: &[f32], sample_rate: u32) -> f32 {
    let len = frame.len();
    // Autocorrelation
    // We look for the lag with the highest correlation in the valid pitch range.
    // Valid range: 50Hz to 500Hz (typical speech).
    // Lag = sample_rate / freq.
    let min_lag = (sample_rate as f32 / 500.0) as usize;
    let max_lag = (sample_rate as f32 / 50.0) as usize;

    // Ensure lags are within frame bounds (natural panic if frame is tiny, but frame_len is usually 1024)
    // If frame is smaller than max_lag, we can't detect 50Hz.
    // We'll just clamp the search range to the frame size naturally.
    let search_end = max_lag.min(len / 2);
    let search_start = min_lag.min(search_end);

    let mut best_corr = -1.0;
    let mut best_lag = 0;

    for lag in search_start..=search_end {
        let mut corr = 0.0;
        // Simple non-normalized autocorrelation
        for i in 0..(len - lag) {
            corr += frame[i] * frame[i + lag];
        }
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    if best_lag > 0 {
        sample_rate as f32 / best_lag as f32
    } else {
        0.0 // No pitch detected
    }
}

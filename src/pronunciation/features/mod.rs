const FRAME_LENGTH_SAMPLES_AT_16K: usize = 1024;
const HOP_SAMPLES_AT_16K: usize = 160; // 10ms at 16kHz
const MIN_FREQUENCY_HZ: f32 = 80.0;
const MAX_FREQUENCY_HZ: f32 = 800.0;

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
    pub frame_starts: Vec<usize>,
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
        for start in (hop..samples.len()).step_by(hop) {
            if start + frame_len > samples.len() {
                break;
            }
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
        let mut start = 0usize;
        loop {
            let frame = &window[start..start + frame_len];
            let chunk_start = start as isize - (tail_len - hop as isize);
            if chunk_start >= 0 {
                frame_starts.push(chunk_start as usize);
                energy.push(frame_energy(frame));
                pitch.push(frame_pitch(frame, sample_rate));
            }
            start += hop;
            if start + frame_len > window.len() {
                break;
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
    let min_period = (sample_rate as f32 / MAX_FREQUENCY_HZ) as usize;
    let max_period = (sample_rate as f32 / MIN_FREQUENCY_HZ) as usize;
    let mut best_lag = min_period;
    let mut best_corr = autocorrelation(frame, min_period);
    for lag in (min_period + 1)..=max_period {
        let corr = autocorrelation(frame, lag);
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }
    sample_rate as f32 / best_lag as f32
}

fn autocorrelation(frame: &[f32], lag: usize) -> f32 {
    frame
        .iter()
        .zip(frame.iter().skip(lag))
        .map(|(a, b)| a * b)
        .sum()
}

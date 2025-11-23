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
        assert!(sample_rate > 0, "sample rate must be positive");
        let frame_len_samples = scaled_samples(FRAME_LENGTH_SAMPLES_AT_16K, sample_rate);
        let hop_samples = scaled_samples(HOP_SAMPLES_AT_16K, sample_rate);
        assert!(
            frame_len_samples > hop_samples,
            "frame length must exceed hop to allow overlap"
        );
        Self {
            frame_len_samples,
            hop_samples,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceFeatures {
    pub energy: Vec<f32>,
    pub pitch: Vec<f32>,
    pub frame_starts: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct ChunkFeatures {
    pub energy: Vec<f32>,
    pub pitch: Vec<f32>,
    pub frame_starts: Vec<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
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
        assert!(sample_rate > 0, "sample rate must be positive");
        let starts = frame_starts(samples.len(), cfg.frame_len_samples, cfg.hop_samples, 0);
        assert!(
            !starts.is_empty(),
            "insufficient samples ({}) for a single frame ({})",
            samples.len(),
            cfg.frame_len_samples
        );
        let energy = starts
            .iter()
            .map(|&start| frame_energy(samples, start, cfg.frame_len_samples))
            .collect();
        let pitch = starts
            .iter()
            .map(|&start| frame_pitch(samples, start, cfg.frame_len_samples, sample_rate))
            .collect();
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
        assert!(sample_rate > 0, "sample rate must be positive");
        assert!(!chunk.is_empty(), "chunk must not be empty");
        let expected_tail = cfg
            .frame_len_samples
            .checked_sub(cfg.hop_samples)
            .expect("frame length must exceed hop");
        assert_eq!(
            prev_tail.len(),
            expected_tail,
            "prev_tail must be frame_len - hop"
        );

        let mut window = Vec::with_capacity(prev_tail.len() + chunk.len());
        window.extend_from_slice(prev_tail);
        window.extend_from_slice(chunk);

        // Frames start every hop, and we only keep windows fully contained in the tail+chunk buffer.
        let combined_starts = frame_starts(
            window.len(),
            cfg.frame_len_samples,
            cfg.hop_samples,
            prev_tail.len(),
        );
        assert!(
            !combined_starts.is_empty(),
            "insufficient samples for one frame (have {}, need {})",
            window.len(),
            cfg.frame_len_samples
        );
        let energy = combined_starts
            .iter()
            .map(|&start| frame_energy(&window, start, cfg.frame_len_samples))
            .collect();
        let pitch = combined_starts
            .iter()
            .map(|&start| frame_pitch(&window, start, cfg.frame_len_samples, sample_rate))
            .collect();
        let frame_starts: Vec<usize> = combined_starts
            .into_iter()
            .map(|start| start - prev_tail.len())
            .collect();

        ChunkFeatures {
            energy,
            pitch,
            frame_starts,
        }
    }
}

fn scaled_samples(base_at_16k: usize, sample_rate: u32) -> usize {
    (((base_at_16k as u64) * sample_rate as u64) / 16_000).max(1) as usize
}

fn frame_starts(total_len: usize, frame_len: usize, hop: usize, first_start: usize) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut start = first_start;
    // Frame i starts at start_i = first_start + i * hop; require start_i + frame_len <= total_len.
    while start + frame_len <= total_len {
        starts.push(start);
        start += hop;
    }
    starts
}

fn frame_energy(samples: &[f32], start: usize, frame_len: usize) -> f32 {
    let frame = &samples[start..start + frame_len];
    let sum_squares: f32 = frame.iter().map(|value| value * value).sum();
    (sum_squares / frame_len as f32).sqrt()
}

fn frame_pitch(samples: &[f32], start: usize, frame_len: usize, sample_rate: u32) -> f32 {
    let frame = &samples[start..start + frame_len];
    let min_period = (sample_rate as f32 / MAX_FREQUENCY_HZ) as usize;
    let max_period = (sample_rate as f32 / MIN_FREQUENCY_HZ) as usize;
    assert!(min_period > 0, "sample rate too low for pitch extraction");
    assert!(
        min_period < frame_len,
        "frame too short for minimum period ({} vs frame {})",
        min_period,
        frame_len
    );
    let max_period = max_period.min(frame_len - 1).max(min_period);
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

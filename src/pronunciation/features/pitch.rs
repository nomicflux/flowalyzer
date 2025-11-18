use pyin::{Framing, PYINExecutor, PadMode};

const FMIN_HZ: f64 = 80.0;
const FMAX_HZ: f64 = 800.0;
const FALLBACK_SAMPLE_RATE: u32 = 16_000;
const FALLBACK_FRAME_LENGTH_SAMPLES: usize = 1024;
const FALLBACK_HOP_SAMPLES: usize = 160; // 10ms at 16kHz

pub fn compute_pitch_frames(samples: &[f32]) -> Vec<f32> {
    compute_pitch_frames_with_rate(samples, FALLBACK_SAMPLE_RATE)
}

pub fn compute_pitch_frames_with_rate(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let sr = if sample_rate == 0 {
        FALLBACK_SAMPLE_RATE
    } else {
        sample_rate
    };
    let hop = FALLBACK_HOP_SAMPLES * sr as usize / FALLBACK_SAMPLE_RATE as usize;
    let frame_len = FALLBACK_FRAME_LENGTH_SAMPLES * sr as usize / FALLBACK_SAMPLE_RATE as usize;
    if samples.len() < frame_len {
        return vec![0.0; 1];
    }
    let mut executor =
        PYINExecutor::<f32>::new(FMIN_HZ, FMAX_HZ, sr, frame_len, None, Some(hop), Some(0.1));
    let framing = Framing::Center(PadMode::Constant(0.0));
    let (_, frequencies, _, _) = executor.pyin(samples, 0.0, framing);
    frequencies
        .into_iter()
        .map(|freq| if freq.is_nan() { 0.0 } else { freq })
        .collect()
}

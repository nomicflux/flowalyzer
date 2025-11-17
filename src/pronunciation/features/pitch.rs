use pyin::{Framing, PYINExecutor, PadMode};

const SAMPLE_RATE: u32 = 16_000;
const FMIN_HZ: f64 = 80.0;
const FMAX_HZ: f64 = 800.0;
const FRAME_LENGTH_SAMPLES: usize = 1024;
const HOP_LENGTH_SAMPLES: usize = 160; // 10ms at 16kHz

pub fn compute_pitch_frames(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let mut executor = PYINExecutor::<f32>::new(
        FMIN_HZ,
        FMAX_HZ,
        SAMPLE_RATE,
        FRAME_LENGTH_SAMPLES,
        None,
        Some(HOP_LENGTH_SAMPLES),
        Some(0.1),
    );
    let framing = Framing::Center(PadMode::Constant(0.0));
    let (_, frequencies, _, _) = executor.pyin(samples, 0.0, framing);
    frequencies
        .into_iter()
        .map(|freq| if freq.is_nan() { 0.0 } else { freq })
        .collect()
}

const HOP_SIZE_SAMPLES: usize = 160;

pub fn compute_energy_frames(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    samples
        .chunks(HOP_SIZE_SAMPLES)
        .map(compute_frame_rms)
        .collect()
}

fn compute_frame_rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = frame.iter().map(|sample| sample * sample).sum();
    (sum_squares / frame.len() as f32).sqrt()
}

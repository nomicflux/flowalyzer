/// Simple sine wave generator for test fixtures.
pub fn sine_wave(sample_rate: u32, frequency: f32, duration_secs: f32) -> Vec<f32> {
    let total_samples = (sample_rate as f32 * duration_secs) as usize;
    (0..total_samples)
        .map(|i| {
            (2.0 * std::f32::consts::PI * frequency * i as f32 / sample_rate as f32)
                .sin()
                .clamp(-1.0, 1.0)
        })
        .collect()
}

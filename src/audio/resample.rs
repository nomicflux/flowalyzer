use anyhow::{ensure, Result};

/// Linearly resample `samples` from `source_rate` to `target_rate`.
pub fn linear_resample(samples: &[f32], source_rate: u32, target_rate: u32) -> Result<Vec<f32>> {
    ensure!(source_rate > 0, "source sample rate must be positive");
    ensure!(target_rate > 0, "target sample rate must be positive");
    if samples.is_empty() || source_rate == target_rate {
        return Ok(samples.to_vec());
    }
    let ratio = target_rate as f32 / source_rate as f32;
    let output_len = ((samples.len() as f32) * ratio).ceil().max(1.0) as usize;
    let mut output = Vec::with_capacity(output_len);
    let last_index = samples.len() - 1;
    for i in 0..output_len {
        let position = i as f32 / ratio;
        let left = position.floor() as usize;
        let right = (left + 1).min(last_index);
        let t = position - left as f32;
        let sample = samples[left] * (1.0 - t) + samples[right] * t;
        output.push(sample);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::linear_resample;

    #[test]
    fn preserves_constant_signal_after_resample() {
        let input = vec![0.5; 480];
        let resampled = linear_resample(&input, 48_000, 16_000).unwrap();
        let expected_len = ((input.len() as f32) * 16_000_f32 / 48_000_f32).ceil() as usize;
        assert_eq!(resampled.len(), expected_len);
        assert!(resampled.iter().all(|&sample| (sample - 0.5).abs() < 1e-6));
    }
}

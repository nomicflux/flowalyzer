//! Speed change operation - time-stretch audio without changing pitch
//!
//! Pure function module following "bricks & studs" philosophy:
//! - Takes AudioChunk and speed_factor as input
//! - Returns time-stretched AudioChunk
//! - No side effects
//! - Uses ssstretch (Signalsmith Stretch) for high-quality pitch-preserving time-stretch

use crate::types::AudioChunk;
use ssstretch::Stretch;

/// Changes the speed of an audio chunk without changing pitch
///
/// # Arguments
/// * `chunk` - The audio chunk to time-stretch
/// * `speed_factor` - Speed multiplier (< 1.0 = slower, 1.0 = unchanged, > 1.0 = faster)
///
/// # Returns
/// AudioChunk with time-stretched audio
///
/// # Examples
/// ```
/// use flowalyzer::types::AudioChunk;
/// use flowalyzer::operations::speed::change_speed;
///
/// let chunk = AudioChunk {
///     samples: vec![1.0, 0.5, 0.0, -0.5, -1.0],
///     sample_rate: 44100,
///     start_time: 0.0,
///     end_time: 0.1,
/// };
///
/// // Make it 50% slower (2x longer)
/// let slower = change_speed(&chunk, 0.5);
/// assert!(slower.samples.len() > chunk.samples.len());
/// ```
pub fn change_speed(chunk: &AudioChunk, speed_factor: f32) -> AudioChunk {
    if is_identity_speed(speed_factor) {
        return chunk.clone();
    }

    let mut stretch = configured_stretch(chunk.sample_rate);
    let samples = collect_stretched_samples(&mut stretch, &chunk.samples, speed_factor);
    let new_duration = samples.len() as f64 / chunk.sample_rate as f64;

    AudioChunk {
        samples,
        sample_rate: chunk.sample_rate,
        start_time: chunk.start_time,
        end_time: chunk.start_time + new_duration,
    }
}

fn is_identity_speed(speed_factor: f32) -> bool {
    (speed_factor - 1.0).abs() < 1e-6
}

fn configured_stretch(sample_rate: u32) -> Stretch {
    let mut stretch = Stretch::new();
    stretch.preset_default(1, sample_rate as f32);
    stretch
}

fn collect_stretched_samples(stretch: &mut Stretch, input: &[f32], speed_factor: f32) -> Vec<f32> {
    let desired_output = (((input.len() as f64) / (speed_factor as f64)).ceil() as usize).max(1);
    let latency = stretch.output_latency().max(0) as usize;
    let output_request = desired_output + latency;
    let inputs = vec![input.to_vec()];
    let mut outputs = vec![Vec::new()];
    stretch.process_vec(
        &inputs,
        input.len() as i32,
        &mut outputs,
        output_request as i32,
    );

    let mut samples = outputs.into_iter().next().unwrap_or_default();
    let mut flush_outputs = vec![Vec::new()];
    stretch.flush_vec(&mut flush_outputs, latency as i32);
    if let Some(mut channel) = flush_outputs.into_iter().next() {
        samples.append(&mut channel);
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chunk(num_samples: usize) -> AudioChunk {
        // Create a simple sine wave for testing
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / num_samples as f32;
            samples.push((t * 2.0 * std::f32::consts::PI * 10.0).sin());
        }

        AudioChunk {
            samples,
            sample_rate: 44100,
            start_time: 0.0,
            end_time: num_samples as f64 / 44100.0,
        }
    }

    #[test]
    fn test_speed_identity() {
        let chunk = create_test_chunk(1000);
        let result = change_speed(&chunk, 1.0);

        assert_eq!(result.sample_rate, chunk.sample_rate);
        assert!(
            result.samples.len() >= chunk.samples.len(),
            "Identity stretch should not return fewer samples (identity_len={}, chunk_len={})",
            result.samples.len(),
            chunk.samples.len()
        );
    }

    #[test]
    fn test_speed_slower() {
        let chunk = create_test_chunk(1000);
        let slower = change_speed(&chunk, 0.5); // 50% speed = 2x longer

        assert_eq!(slower.sample_rate, 44100);
        assert!(
            slower.samples.len() >= chunk.samples.len(),
            "Expected slower audio to have at least as many samples (slow_len={} chunk_len={})",
            slower.samples.len(),
            chunk.samples.len()
        );
        assert!(
            slower.samples.iter().any(|s| s.abs() > 0.0),
            "Expected slower output to contain non-zero samples"
        );
    }

    #[test]
    fn test_speed_faster() {
        let chunk = create_test_chunk(1000);
        let faster = change_speed(&chunk, 2.0); // 200% speed = 0.5x length

        assert_eq!(faster.sample_rate, 44100);
        assert!(
            faster.samples.iter().any(|s| s.abs() > 0.0),
            "Expected faster output to contain non-zero samples"
        );
    }

    #[test]
    fn test_speed_preserves_sample_rate() {
        let chunk = AudioChunk {
            samples: vec![0.1; 500],
            sample_rate: 48000,
            start_time: 0.0,
            end_time: 0.5,
        };

        let result = change_speed(&chunk, 1.5);
        assert_eq!(result.sample_rate, 48000);
    }

    #[test]
    fn test_speed_various_factors() {
        let chunk = create_test_chunk(800);

        for &factor in &[0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0] {
            let result = change_speed(&chunk, factor);
            let len = result.samples.len();
            assert!(
                len > 0,
                "Expected non-empty output for speed_factor = {}",
                factor
            );
            assert_eq!(
                result.sample_rate, chunk.sample_rate,
                "Expected sample rate to remain unchanged for speed_factor = {}",
                factor
            );
        }
    }

    #[test]
    fn test_speed_fast_preserves_tail_energy() {
        let mut chunk = create_test_chunk(1500);
        let last_index = chunk.samples.len().saturating_sub(1);
        chunk.samples[last_index] = 0.8; // Ensure non-zero tail

        let result = change_speed(&chunk, 2.5); // Fast playback
        let tail_energy: f32 = result.samples.iter().rev().take(100).map(|s| s.abs()).sum();
        let tail_samples: Vec<f32> = result.samples.iter().rev().take(10).cloned().collect();
        assert!(
            tail_energy > 0.05,
            "Fast playback tail energy too low: {} tail_samples={:?}",
            tail_energy,
            tail_samples
        );
    }
}

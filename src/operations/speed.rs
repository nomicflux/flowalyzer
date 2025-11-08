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
///     metadata: None,
/// };
///
/// // Make it 50% slower (2x longer)
/// let slower = change_speed(&chunk, 0.5);
/// assert!(slower.samples.len() > chunk.samples.len());
/// ```
pub fn change_speed(chunk: &AudioChunk, speed_factor: f32) -> AudioChunk {
    // Edge case: speed_factor = 1.0, return identical chunk
    if (speed_factor - 1.0).abs() < 1e-6 {
        return chunk.clone();
    }

    // Create and configure mono stretcher
    let mut stretch = Stretch::new();
    stretch.preset_default(1, chunk.sample_rate as f32); // 1 channel, chunk's sample rate

    // Calculate output length based on speed factor
    // speed_factor < 1.0 = slower = longer output
    // speed_factor > 1.0 = faster = shorter output
    let output_len = (chunk.samples.len() as f32 / speed_factor).round() as usize;

    // Prepare input and output buffers as Vec<Vec<f32>> (channel arrays)
    let inputs = vec![chunk.samples.clone()];
    let mut outputs = vec![Vec::new()]; // Will be resized by process_vec

    // Process audio through stretcher
    stretch.process_vec(
        &inputs,                    // Input: slice of channel vecs
        chunk.samples.len() as i32, // Input length
        &mut outputs,               // Output: mutable slice of channel vecs
        output_len as i32,          // Output length
    );

    // Extract processed samples from first (and only) channel
    let output_samples = outputs.into_iter().next().unwrap();

    // Calculate new duration
    let new_duration = output_samples.len() as f64 / chunk.sample_rate as f64;

    AudioChunk {
        samples: output_samples,
        sample_rate: chunk.sample_rate,
        start_time: chunk.start_time,
        end_time: chunk.start_time + new_duration,
        metadata: None,
    }
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
            metadata: None,
        }
    }

    #[test]
    fn test_speed_identity() {
        let chunk = create_test_chunk(1000);
        let result = change_speed(&chunk, 1.0);

        // Should return same length for speed_factor = 1.0
        assert_eq!(result.samples.len(), chunk.samples.len());
        assert_eq!(result.sample_rate, chunk.sample_rate);
    }

    #[test]
    fn test_speed_slower() {
        let chunk = create_test_chunk(1000);
        let result = change_speed(&chunk, 0.5); // 50% speed = 2x longer

        // Output should be approximately 2x longer
        let expected_len = (1000.0_f32 / 0.5).round() as usize;
        assert_eq!(result.samples.len(), expected_len);
        assert_eq!(result.sample_rate, 44100);

        // Duration should be ~2x longer
        let original_duration = chunk.end_time - chunk.start_time;
        let new_duration = result.end_time - result.start_time;
        assert!((new_duration / original_duration - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_speed_faster() {
        let chunk = create_test_chunk(1000);
        let result = change_speed(&chunk, 2.0); // 200% speed = 0.5x length

        // Output should be approximately 0.5x length
        let expected_len = (1000.0_f32 / 2.0).round() as usize;
        assert_eq!(result.samples.len(), expected_len);
        assert_eq!(result.sample_rate, 44100);

        // Duration should be ~0.5x original
        let original_duration = chunk.end_time - chunk.start_time;
        let new_duration = result.end_time - result.start_time;
        assert!((new_duration / original_duration - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_speed_preserves_sample_rate() {
        let chunk = AudioChunk {
            samples: vec![0.1; 500],
            sample_rate: 48000,
            start_time: 0.0,
            end_time: 0.5,
            metadata: None,
        };

        let result = change_speed(&chunk, 1.5);
        assert_eq!(result.sample_rate, 48000);
    }

    #[test]
    fn test_speed_various_factors() {
        let chunk = create_test_chunk(800);

        // Test various speed factors
        for &factor in &[0.25, 0.5, 0.75, 1.5, 2.0, 3.0] {
            let result = change_speed(&chunk, factor);
            let expected_len = (800.0 / factor).round() as usize;
            assert_eq!(
                result.samples.len(),
                expected_len,
                "Failed for speed_factor = {}",
                factor
            );
        }
    }
}

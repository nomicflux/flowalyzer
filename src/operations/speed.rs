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
    if input.is_empty() {
        return Vec::new();
    }

    let input_latency = stretch.input_latency().max(0) as usize;
    let output_latency = stretch.output_latency().max(0) as usize;
    let block_samples = stretch.block_samples().max(0) as usize;

    let stretch_ratio = (1.0 / speed_factor) as f64;
    let main_output_len = ((input.len() as f64 + block_samples as f64) * stretch_ratio)
        .ceil()
        .max(1.0) as i32;

    let inputs = vec![input.to_vec()];
    let mut outputs = vec![vec![0.0f32; main_output_len as usize]];
    stretch.process_vec(&inputs, input.len() as i32, &mut outputs, main_output_len);
    let mut samples = outputs.remove(0);

    if input_latency > 0 {
        let pad_input = vec![vec![0.0f32; input_latency]];
        let pad_output_len = ((input_latency as f64 + block_samples as f64) * stretch_ratio)
            .ceil()
            .max(1.0) as i32;
        let mut pad_outputs = vec![vec![0.0f32; pad_output_len as usize]];
        stretch.process_vec(
            &pad_input,
            input_latency as i32,
            &mut pad_outputs,
            pad_output_len,
        );
        samples.extend_from_slice(&pad_outputs.remove(0));
    }

    if output_latency > 0 {
        let mut flush_buffer = vec![vec![0.0f32; output_latency]];
        stretch.flush_vec(&mut flush_buffer, output_latency as i32);
        samples.extend_from_slice(&flush_buffer[0]);
    }

    let target_len = ((input.len() as f64) * stretch_ratio).round() as isize;
    let target_len = target_len.clamp(0, samples.len() as isize) as usize;
    samples.truncate(target_len);
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
    fn test_speed_exact_lengths() {
        let chunk = create_test_chunk(1024);
        for &factor in &[0.5, 0.75, 1.25, 1.8, 2.0, 3.0] {
            let stretched = change_speed(&chunk, factor);
            let expected = ((chunk.samples.len() as f64) / factor as f64)
                .round()
                .max(1.0) as usize;
            assert_eq!(
                stretched.samples.len(),
                expected,
                "len mismatch for factor {}",
                factor
            );
        }
    }

    #[test]
    fn test_round_trip_slow_fast() {
        let chunk = create_test_chunk(2048);
        let factor = 0.6;
        let slowed = change_speed(&chunk, factor);
        let back = change_speed(&slowed, 1.0 / factor);
        assert_eq!(chunk.sample_rate, back.sample_rate);
        assert_eq!(
            chunk.samples, back.samples,
            "round-trip slow→fast should return identical samples"
        );
    }

    #[test]
    fn test_round_trip_fast_slow() {
        let chunk = create_test_chunk(2048);
        let factor = 2.5;
        let sped = change_speed(&chunk, factor);
        let back = change_speed(&sped, 1.0 / factor);
        assert_eq!(chunk.sample_rate, back.sample_rate);
        assert_eq!(
            chunk.samples, back.samples,
            "round-trip fast→slow should return identical samples"
        );
    }

    #[test]
    fn test_short_clip_round_trip() {
        let chunk = create_test_chunk(64);
        let factor = 1.8;
        let sped = change_speed(&chunk, factor);
        let back = change_speed(&sped, 1.0 / factor);
        assert_eq!(chunk.sample_rate, back.sample_rate);
        assert_eq!(
            chunk.samples, back.samples,
            "round-trip on short clip should match exactly"
        );
    }
}

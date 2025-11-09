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

    let mut samples = process_block(
        stretch,
        input,
        compute_output_len(input.len(), speed_factor),
    );

    let input_latency = stretch.input_latency().max(0) as usize;
    if input_latency > 0 {
        append_silence_block(stretch, speed_factor, input_latency, &mut samples);
    }

    let output_latency = stretch.output_latency().max(0) as usize;
    append_flush(stretch, output_latency, &mut samples);
    remove_pre_roll(&mut samples, output_latency);

    samples
}

fn compute_output_len(input_samples: usize, speed_factor: f32) -> usize {
    ((input_samples as f64) / speed_factor as f64)
        .round()
        .max(1.0) as usize
}

fn process_block(stretch: &mut Stretch, input: &[f32], output_len: usize) -> Vec<f32> {
    let mut outputs = vec![vec![0.0f32; output_len.max(1)]];
    let inputs = vec![input.to_vec()];
    let input_len = input.len() as i32;
    let output_len = outputs[0].len() as i32;
    stretch.process_vec(&inputs, input_len, &mut outputs, output_len);
    outputs.remove(0)
}

fn append_silence_block(
    stretch: &mut Stretch,
    speed_factor: f32,
    input_samples: usize,
    buffer: &mut Vec<f32>,
) {
    let silence_len = compute_output_len(input_samples, speed_factor);
    let silence_inputs = vec![vec![0.0f32; input_samples]];
    let mut outputs = vec![vec![0.0f32; silence_len]];
    let input_len = input_samples as i32;
    let output_len = outputs[0].len() as i32;
    stretch.process_vec(&silence_inputs, input_len, &mut outputs, output_len);
    buffer.extend_from_slice(&outputs[0]);
}

fn append_flush(stretch: &mut Stretch, output_latency: usize, buffer: &mut Vec<f32>) {
    if output_latency == 0 {
        return;
    }

    let mut outputs = vec![vec![0.0f32; output_latency]];
    stretch.flush_vec(&mut outputs, output_latency as i32);
    buffer.extend_from_slice(&outputs[0]);
}

fn remove_pre_roll(samples: &mut Vec<f32>, pre_roll: usize) {
    let remove = pre_roll.min(samples.len());
    if remove > 0 {
        samples.drain(0..remove);
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
        }
    }

    #[test]
    fn identity_speed_returns_original_chunk() {
        let chunk = create_test_chunk(1024);
        let result = change_speed(&chunk, 1.0);

        assert_eq!(result.sample_rate, chunk.sample_rate);
        assert_eq!(result.samples, chunk.samples);
    }

    #[test]
    fn slow_then_normal_round_trip_preserves_length() {
        let chunk = create_test_chunk(2048);
        let slow_factor = 0.75;

        let slowed = change_speed(&chunk, slow_factor);
        let back = change_speed(&slowed, 1.0 / slow_factor);

        assert_eq!(back.sample_rate, chunk.sample_rate);
        assert!(
            back.samples.len() >= chunk.samples.len(),
            "slow → normal round-trip should not clip samples"
        );
    }

    #[test]
    fn fast_then_normal_round_trip_preserves_length() {
        let chunk = create_test_chunk(2048);
        let fast_factor = 1.8;

        let sped = change_speed(&chunk, fast_factor);
        let back = change_speed(&sped, 1.0 / fast_factor);

        assert_eq!(back.sample_rate, chunk.sample_rate);
        assert!(
            back.samples.len() >= chunk.samples.len(),
            "fast → normal round-trip should not clip samples"
        );
    }
}

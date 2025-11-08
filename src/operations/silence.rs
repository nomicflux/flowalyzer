//! Silence insertion operation - generates silent audio chunks
//!
//! Pure function module following "bricks & studs" philosophy:
//! - Takes duration and sample_rate as input
//! - Returns AudioChunk with zero samples
//! - No side effects

use crate::types::AudioChunk;

/// Generates a silent audio chunk of specified duration
///
/// # Arguments
/// * `duration` - Duration in seconds (0.0 returns empty samples)
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// AudioChunk containing silent samples (all zeros)
///
/// # Examples
/// ```
/// use flowalyzer::operations::silence::insert_silence;
///
/// // Generate 1 second of silence at 44.1kHz
/// let silence = insert_silence(1.0, 44100);
/// assert_eq!(silence.samples.len(), 44100);
/// assert!(silence.samples.iter().all(|&s| s == 0.0));
/// ```
pub fn insert_silence(duration: f64, sample_rate: u32) -> AudioChunk {
    let num_samples = (duration * sample_rate as f64) as usize;

    AudioChunk {
        samples: vec![0.0; num_samples],
        sample_rate,
        start_time: 0.0,
        end_time: duration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_zero_duration() {
        let silence = insert_silence(0.0, 44100);
        assert_eq!(silence.samples.len(), 0);
        assert_eq!(silence.sample_rate, 44100);
        assert_eq!(silence.start_time, 0.0);
        assert_eq!(silence.end_time, 0.0);
    }

    #[test]
    fn test_silence_short_duration() {
        let silence = insert_silence(0.1, 44100);
        let expected_samples = (0.1 * 44100.0) as usize; // 4410 samples
        assert_eq!(silence.samples.len(), expected_samples);
        assert_eq!(silence.sample_rate, 44100);
        assert_eq!(silence.end_time, 0.1);

        // All samples should be zero
        assert!(silence.samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_silence_one_second() {
        let silence = insert_silence(1.0, 44100);
        assert_eq!(silence.samples.len(), 44100);
        assert_eq!(silence.sample_rate, 44100);
        assert_eq!(silence.end_time, 1.0);

        // All samples should be zero
        assert!(silence.samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_silence_different_sample_rate() {
        let silence = insert_silence(0.5, 48000);
        let expected_samples = (0.5 * 48000.0) as usize; // 24000 samples
        assert_eq!(silence.samples.len(), expected_samples);
        assert_eq!(silence.sample_rate, 48000);
        assert_eq!(silence.end_time, 0.5);

        // All samples should be zero
        assert!(silence.samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_silence_very_short() {
        let silence = insert_silence(0.001, 44100); // 1ms
        let expected_samples = (0.001 * 44100.0) as usize; // ~44 samples
        assert_eq!(silence.samples.len(), expected_samples);
        assert!(silence.samples.iter().all(|&s| s == 0.0));
    }
}

//! Repeat operation - repeats an audio chunk N times
//!
//! Pure function module following "bricks & studs" philosophy:
//! - Takes AudioChunk and count as input
//! - Returns Vec of cloned chunks
//! - No side effects

use crate::types::AudioChunk;

/// Repeats an audio chunk N times
///
/// # Arguments
/// * `chunk` - The audio chunk to repeat
/// * `count` - Number of times to repeat (0 returns empty vec)
///
/// # Returns
/// Vector containing `count` clones of the input chunk
pub fn repeat_chunk(chunk: &AudioChunk, count: u32) -> Vec<AudioChunk> {
    if count == 0 {
        return Vec::new();
    }
    (0..count).map(|_| chunk.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chunk() -> AudioChunk {
        AudioChunk {
            samples: vec![1.0, 0.5, 0.0, -0.5, -1.0],
            sample_rate: 44100,
            start_time: 0.0,
            end_time: 0.1,
        }
    }

    #[test]
    fn test_repeat_zero() {
        let chunk = create_test_chunk();
        let result = repeat_chunk(&chunk, 0);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_repeat_one() {
        let chunk = create_test_chunk();
        let result = repeat_chunk(&chunk, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].samples, chunk.samples);
        assert_eq!(result[0].sample_rate, chunk.sample_rate);
    }

    #[test]
    fn test_repeat_three() {
        let chunk = create_test_chunk();
        let result = repeat_chunk(&chunk, 3);
        assert_eq!(result.len(), 3);

        for repeated_chunk in result.iter() {
            assert_eq!(repeated_chunk.samples, chunk.samples);
            assert_eq!(repeated_chunk.sample_rate, chunk.sample_rate);
        }
    }
}

pub mod recipe;
pub mod repeat;
pub mod silence;
pub mod speed;

use crate::types::{AudioChunk, Operation};

// Re-export operation functions for convenience
pub use repeat::repeat_chunk;
pub use silence::insert_silence;
pub use speed::change_speed;

/// Apply an operation to an audio chunk
///
/// This dispatcher function routes operations to their specific implementations
/// based on the Operation enum variant.
///
/// # Arguments
/// * `chunk` - The audio chunk to process
/// * `op` - The operation to apply
///
/// # Returns
/// Vector of AudioChunks (some operations like Repeat produce multiple chunks)
pub fn apply_operation(chunk: &AudioChunk, op: &Operation) -> Vec<AudioChunk> {
    match op {
        Operation::Identity => vec![chunk.clone()],
        Operation::Repeat(count) => repeat_chunk(chunk, *count),
        Operation::Speed(factor) => vec![change_speed(chunk, *factor)],
        Operation::InsertSilence(duration) => {
            vec![insert_silence(*duration, chunk.sample_rate)]
        }
    }
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
            metadata: None,
        }
    }

    #[test]
    fn test_apply_identity() {
        let chunk = create_test_chunk();
        let result = apply_operation(&chunk, &Operation::Identity);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].samples, chunk.samples);
    }

    #[test]
    fn test_apply_repeat() {
        let chunk = create_test_chunk();
        let result = apply_operation(&chunk, &Operation::Repeat(3));

        assert_eq!(result.len(), 3);
        for repeated_chunk in &result {
            assert_eq!(repeated_chunk.samples, chunk.samples);
        }
    }

    #[test]
    fn test_apply_speed() {
        let chunk = create_test_chunk();
        let result = apply_operation(&chunk, &Operation::Speed(2.0));

        assert_eq!(result.len(), 1);
        // Faster = shorter
        assert!(result[0].samples.len() < chunk.samples.len());
    }

    #[test]
    fn test_apply_insert_silence() {
        let chunk = create_test_chunk();
        let result = apply_operation(&chunk, &Operation::InsertSilence(0.5));

        assert_eq!(result.len(), 1);
        // Should have 0.5 seconds of silence at 44100 Hz
        assert_eq!(result[0].samples.len(), (0.5 * 44100.0) as usize);
        assert!(result[0].samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_dispatcher_all_variants() {
        let chunk = create_test_chunk();

        // Test all Operation enum variants work
        let operations = vec![
            Operation::Identity,
            Operation::Repeat(2),
            Operation::Speed(1.5),
            Operation::InsertSilence(0.1),
        ];

        for op in operations {
            let result = apply_operation(&chunk, &op);
            assert!(
                !result.is_empty(),
                "Operation {:?} returned empty result",
                op
            );
        }
    }
}

use crate::types::{AudioChunk, AudioData, ChunkBoundary};

/// Pure function to slice audio data into chunks based on time boundaries
pub fn slice_audio(audio: &AudioData, boundaries: &[ChunkBoundary]) -> Vec<AudioChunk> {
    let mut chunks = Vec::with_capacity(boundaries.len());

    for boundary in boundaries {
        // Convert time boundaries to sample indices
        let start_sample = (boundary.start_time * audio.sample_rate as f64) as usize;
        let end_sample = (boundary.end_time * audio.sample_rate as f64) as usize;

        // Clamp to valid range
        let start_sample = start_sample.min(audio.samples.len());
        let end_sample = end_sample.min(audio.samples.len());

        // Extract samples for this chunk
        let samples = audio.samples[start_sample..end_sample].to_vec();

        chunks.push(AudioChunk {
            samples,
            sample_rate: audio.sample_rate,
            start_time: boundary.start_time,
            end_time: boundary.end_time,
            metadata: None,
        });
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_slicing() {
        // Create test audio: 1 second at 44100 Hz
        let audio = AudioData {
            samples: vec![0.0; 44100],
            sample_rate: 44100,
        };

        let boundaries = vec![
            ChunkBoundary {
                start_time: 0.0,
                end_time: 0.5,
                source_segment_ids: vec![0],
            },
            ChunkBoundary {
                start_time: 0.5,
                end_time: 1.0,
                source_segment_ids: vec![1],
            },
        ];

        let chunks = slice_audio(&audio, &boundaries);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].samples.len(), 22050); // 0.5 * 44100
        assert_eq!(chunks[1].samples.len(), 22050);
    }

    #[test]
    fn test_boundary_clamping() {
        let audio = AudioData {
            samples: vec![0.0; 1000],
            sample_rate: 1000,
        };

        // Boundary extends beyond audio length
        let boundaries = vec![ChunkBoundary {
            start_time: 0.5,
            end_time: 2.0, // Beyond 1 second
            source_segment_ids: vec![0],
        }];

        let chunks = slice_audio(&audio, &boundaries);

        assert_eq!(chunks.len(), 1);
        // Should clamp to available samples
        assert_eq!(chunks[0].samples.len(), 500); // 0.5 seconds worth
    }
}

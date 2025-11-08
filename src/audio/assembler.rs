use crate::types::{AudioChunk, AudioData};

/// Pure function to concatenate audio chunks into single continuous audio
/// Adds a simple crossfade between chunks to prevent clicks
pub fn assemble_audio(chunks: &[AudioChunk]) -> Option<AudioData> {
    if chunks.is_empty() {
        return None;
    }

    // Verify all chunks have the same sample rate
    let sample_rate = chunks[0].sample_rate;
    if !chunks.iter().all(|c| c.sample_rate == sample_rate) {
        return None; // Mismatched sample rates
    }

    // Calculate crossfade length (2ms to prevent clicks)
    let crossfade_samples = ((sample_rate as f64 * 0.002) as usize).max(10);

    // Estimate total size
    let total_samples: usize = chunks.iter().map(|c| c.samples.len()).sum();
    let mut assembled = Vec::with_capacity(total_samples);

    for (idx, chunk) in chunks.iter().enumerate() {
        if idx == 0 {
            // First chunk: add all samples
            assembled.extend_from_slice(&chunk.samples);
        } else {
            // Subsequent chunks: crossfade with previous chunk
            let overlap_len = crossfade_samples
                .min(chunk.samples.len())
                .min(assembled.len());

            if overlap_len > 0 {
                // Apply crossfade: fade out previous, fade in current
                let fade_start = assembled.len() - overlap_len;

                for i in 0..overlap_len {
                    let t = i as f32 / overlap_len as f32; // 0.0 to 1.0
                    let prev_weight = 1.0 - t;
                    let curr_weight = t;

                    // Mix the overlapping samples
                    assembled[fade_start + i] =
                        assembled[fade_start + i] * prev_weight + chunk.samples[i] * curr_weight;
                }

                // Add remaining samples from this chunk
                assembled.extend_from_slice(&chunk.samples[overlap_len..]);
            } else {
                // No overlap possible, just concatenate
                assembled.extend_from_slice(&chunk.samples);
            }
        }
    }

    Some(AudioData {
        samples: assembled,
        sample_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_assembly() {
        let chunks = vec![
            AudioChunk {
                samples: vec![1.0; 100],
                sample_rate: 44100,
                start_time: 0.0,
                end_time: 0.1,
                metadata: None,
            },
            AudioChunk {
                samples: vec![0.5; 100],
                sample_rate: 44100,
                start_time: 0.1,
                end_time: 0.2,
                metadata: None,
            },
        ];

        let result = assemble_audio(&chunks);
        assert!(result.is_some());

        let audio = result.unwrap();
        assert_eq!(audio.sample_rate, 44100);
        // Total should be less than 200 due to crossfade overlap
        assert!(audio.samples.len() < 200);
        assert!(audio.samples.len() > 100);
    }

    #[test]
    fn test_empty_chunks() {
        let chunks: Vec<AudioChunk> = vec![];
        let result = assemble_audio(&chunks);
        assert!(result.is_none());
    }

    #[test]
    fn test_mismatched_sample_rates() {
        let chunks = vec![
            AudioChunk {
                samples: vec![1.0; 100],
                sample_rate: 44100,
                start_time: 0.0,
                end_time: 0.1,
                metadata: None,
            },
            AudioChunk {
                samples: vec![0.5; 100],
                sample_rate: 48000, // Different!
                start_time: 0.1,
                end_time: 0.2,
                metadata: None,
            },
        ];

        let result = assemble_audio(&chunks);
        assert!(result.is_none()); // Should reject mismatched rates
    }
}

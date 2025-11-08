use crate::types::{ChunkBoundary, ChunkConfig, Transcript};

/// Pure function to determine chunk boundaries from transcript
/// Tries to create chunks close to target duration by combining segments
pub fn calculate_chunk_boundaries(
    transcript: &Transcript,
    config: ChunkConfig,
) -> Vec<ChunkBoundary> {
    let mut boundaries = Vec::new();
    let mut current_start = 0.0;
    let mut current_segments = Vec::new();

    for (idx, segment) in transcript.segments.iter().enumerate() {
        let segment_duration = segment.end_time - segment.start_time;

        // If this segment alone exceeds max duration, split it
        if segment_duration > config.max_duration {
            // Finalize current chunk if any
            if !current_segments.is_empty() {
                let last_seg_idx: usize = current_segments[current_segments.len() - 1];
                boundaries.push(ChunkBoundary {
                    start_time: current_start,
                    end_time: transcript.segments[last_seg_idx].end_time,
                    source_segment_ids: current_segments.clone(),
                });
                current_segments.clear();
            }

            // Split long segment into multiple chunks at target duration
            let mut seg_start = segment.start_time;
            while seg_start < segment.end_time {
                let chunk_end = (seg_start + config.target_duration).min(segment.end_time);
                boundaries.push(ChunkBoundary {
                    start_time: seg_start,
                    end_time: chunk_end,
                    source_segment_ids: vec![idx],
                });
                seg_start = chunk_end;
            }

            current_start = segment.end_time;
            continue;
        }

        // Calculate what duration we'd have if we added this segment
        let potential_end = segment.end_time;
        let potential_duration = potential_end - current_start;

        // If adding this segment would exceed max duration, finalize current chunk
        if !current_segments.is_empty() && potential_duration > config.max_duration {
            let last_seg_idx: usize = current_segments[current_segments.len() - 1];
            boundaries.push(ChunkBoundary {
                start_time: current_start,
                end_time: transcript.segments[last_seg_idx].end_time,
                source_segment_ids: current_segments.clone(),
            });
            current_segments.clear();
            current_start = segment.start_time;
        }

        // Add this segment to current chunk
        if current_segments.is_empty() {
            current_start = segment.start_time;
        }
        current_segments.push(idx);

        // If we've reached or exceeded target duration, finalize chunk
        let current_duration = segment.end_time - current_start;
        if current_duration >= config.min_duration {
            // Good enough, finalize this chunk
            boundaries.push(ChunkBoundary {
                start_time: current_start,
                end_time: segment.end_time,
                source_segment_ids: current_segments.clone(),
            });
            current_segments.clear();
            current_start = segment.end_time;
        }
    }

    // Finalize any remaining segments
    if !current_segments.is_empty() {
        let last_seg_idx: usize = current_segments[current_segments.len() - 1];
        boundaries.push(ChunkBoundary {
            start_time: current_start,
            end_time: transcript.segments[last_seg_idx].end_time,
            source_segment_ids: current_segments,
        });
    }

    boundaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Granularity, Segment};

    #[test]
    fn test_basic_chunking() {
        let transcript = Transcript {
            segments: vec![
                Segment {
                    text: "Hello".to_string(),
                    start_time: 0.0,
                    end_time: 0.5,
                    granularity: Granularity::Word,
                },
                Segment {
                    text: "world".to_string(),
                    start_time: 0.5,
                    end_time: 1.0,
                    granularity: Granularity::Word,
                },
                Segment {
                    text: "This is a test".to_string(),
                    start_time: 1.5,
                    end_time: 2.5,
                    granularity: Granularity::Sentence,
                },
            ],
        };

        let config = ChunkConfig::new(2.0);
        let boundaries = calculate_chunk_boundaries(&transcript, config);

        assert!(!boundaries.is_empty());
        // Should create at least one chunk
        assert!(boundaries[0].end_time > boundaries[0].start_time);
    }

    #[test]
    fn test_long_segment_splitting() {
        let transcript = Transcript {
            segments: vec![Segment {
                text: "Very long sentence".to_string(),
                start_time: 0.0,
                end_time: 5.0, // Exceeds max duration
                granularity: Granularity::Sentence,
            }],
        };

        let config = ChunkConfig::new(2.0);
        let boundaries = calculate_chunk_boundaries(&transcript, config);

        // Should split into multiple chunks
        assert!(boundaries.len() > 1);
    }
}

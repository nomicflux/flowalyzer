use crate::types::{ChunkBoundary, ChunkConfig, SampleRate, Transcript};

use super::accumulator::ChunkAccumulator;
use super::spans::build_spans;

/// Pure function to determine chunk boundaries from transcript
/// Tries to create chunks close to target duration by combining segments
pub fn calculate_chunk_boundaries(
    transcript: &Transcript,
    config: ChunkConfig,
    pauses: &[f64],
    sample_rate: SampleRate,
) -> Vec<ChunkBoundary> {
    let spans = build_spans(transcript, pauses, sample_rate);
    let mut accumulator = ChunkAccumulator::new(sample_rate);
    for span in spans {
        accumulator.handle_span(span, config);
    }
    accumulator.finish_chunk();
    accumulator.into_boundaries()
}

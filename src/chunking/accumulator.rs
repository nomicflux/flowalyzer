use crate::types::{ChunkBoundary, ChunkConfig};

use super::spans::{Span, EPS};

pub(super) struct ChunkAccumulator {
    boundaries: Vec<ChunkBoundary>,
    current_start: f64,
    current_end: f64,
    current_segments: Vec<usize>,
}

impl ChunkAccumulator {
    pub(super) fn new() -> Self {
        Self {
            boundaries: Vec::new(),
            current_start: 0.0,
            current_end: 0.0,
            current_segments: Vec::new(),
        }
    }

    pub(super) fn handle_span(&mut self, span: Span, config: ChunkConfig) {
        if span.duration() <= EPS {
            return;
        }
        if self.split_if_excessive(span, config) {
            return;
        }
        self.make_room_for(span, config);
        self.attach_span(span);
        self.finalize_if_ready(config.min_duration);
    }

    pub(super) fn finish_chunk(&mut self) {
        if self.current_segments.is_empty() {
            return;
        }
        self.boundaries.push(ChunkBoundary {
            start_time: self.current_start,
            end_time: self.current_end,
            source_segment_ids: self.current_segments.clone(),
        });
        self.current_segments.clear();
        self.current_start = self.current_end;
    }

    pub(super) fn into_boundaries(self) -> Vec<ChunkBoundary> {
        self.boundaries
    }

    fn split_if_excessive(&mut self, span: Span, config: ChunkConfig) -> bool {
        if span.duration() <= config.max_duration + config.max_overshoot {
            return false;
        }
        self.finish_chunk();
        let mut seg_start = span.start_time;
        while seg_start < span.end_time - EPS {
            let chunk_end = (seg_start + config.target_duration).min(span.end_time);
            self.boundaries.push(ChunkBoundary {
                start_time: seg_start,
                end_time: chunk_end,
                source_segment_ids: vec![span.segment_idx],
            });
            seg_start = chunk_end;
        }
        self.reset_to(span.end_time);
        true
    }

    fn make_room_for(&mut self, span: Span, config: ChunkConfig) {
        if self.current_segments.is_empty() {
            return;
        }
        let potential_duration = span.end_time - self.current_start;
        if potential_duration <= config.max_duration {
            return;
        }
        let max_allowed = config.max_duration + config.max_overshoot;
        if potential_duration <= max_allowed {
            return;
        }
        self.finish_chunk();
        self.reset_to(span.start_time);
    }

    fn attach_span(&mut self, span: Span) {
        if self.current_segments.is_empty() {
            self.current_start = span.start_time;
        }
        if self.current_segments.last().copied() != Some(span.segment_idx) {
            self.current_segments.push(span.segment_idx);
        }
        self.current_end = span.end_time;
    }

    fn finalize_if_ready(&mut self, min_duration: f64) {
        if self.duration() >= min_duration {
            self.finish_chunk();
        }
    }

    fn reset_to(&mut self, start: f64) {
        self.current_start = start;
        self.current_end = start;
        self.current_segments.clear();
    }

    fn duration(&self) -> f64 {
        if self.current_segments.is_empty() {
            0.0
        } else {
            self.current_end - self.current_start
        }
    }
}

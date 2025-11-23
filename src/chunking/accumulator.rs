use crate::types::{ChunkBoundary, ChunkConfig, FrameCount, FrameIndex, FrameRange, SampleRate};

use super::spans::{Span, EPS};

pub(super) struct ChunkAccumulator {
    boundaries: Vec<ChunkBoundary>,
    current_frame_start: Option<FrameIndex>,
    current_frame_end: FrameIndex,
    current_segments: Vec<usize>,
    sample_rate: SampleRate,
}

impl ChunkAccumulator {
    pub(super) fn new(sample_rate: SampleRate) -> Self {
        Self {
            boundaries: Vec::new(),
            current_frame_start: None,
            current_frame_end: FrameIndex::ZERO,
            current_segments: Vec::new(),
            sample_rate,
        }
    }

    pub(super) fn handle_span(&mut self, span: Span, config: ChunkConfig) {
        if span.duration() <= EPS || span.frame_length() == 0 {
            return;
        }
        if self.split_if_excessive(span, config) {
            return;
        }
        // If current chunk has reached target, finalize it before adding new span
        if !self.current_segments.is_empty()
            && self.duration_frames().as_u64() >= config.target_frames.as_u64()
        {
            self.finish_chunk();
        }
        // If adding this span would exceed max, finalize current chunk first
        if !self.current_segments.is_empty() {
            let potential_frames = span
                .frame_range
                .end()
                .saturating_sub(self.current_frame_start.expect("start set when non-empty"));
            let max_allowed = config.max_frames + config.overshoot_frames;
            if potential_frames.as_u64() > max_allowed.as_u64() {
                self.finish_chunk();
            }
        }
        self.attach_span(span);
        // Check if we've reached target after adding the span
        if self.duration_frames().as_u64() >= config.target_frames.as_u64() {
            self.finish_chunk();
        }
    }

    pub(super) fn finish_chunk(&mut self) {
        if self.current_segments.is_empty() || self.current_frame_start.is_none() {
            return;
        }
        let start = self.current_frame_start.unwrap();
        let frame_range = FrameRange::new(
            start,
            FrameCount::from(
                self.current_frame_end
                    .as_usize()
                    .saturating_sub(start.as_usize()),
            ),
        );
        self.boundaries.push(ChunkBoundary {
            start_time: self.sample_rate.seconds_from_frames(start.to_count()),
            end_time: self
                .sample_rate
                .seconds_from_frames(frame_range.end().to_count()),
            frame_range,
            source_segment_ids: self.current_segments.clone(),
        });
        self.current_segments.clear();
        self.current_frame_start = None;
        self.current_frame_end = frame_range.end();
    }

    pub(super) fn into_boundaries(self) -> Vec<ChunkBoundary> {
        self.boundaries
    }

    fn split_if_excessive(&mut self, span: Span, config: ChunkConfig) -> bool {
        if span.frame_range.length.as_u64()
            <= (config.max_frames + config.overshoot_frames).as_u64()
        {
            return false;
        }
        self.finish_chunk();
        let mut seg_start = span.frame_range.start;
        let mut remaining = span.frame_range.length;
        while remaining.as_usize() > 0 {
            let chunk_len =
                FrameCount::from(remaining.as_usize().min(config.target_frames.as_usize()));
            let chunk_end = FrameIndex::from(seg_start.as_usize() + chunk_len.as_usize());
            let frame_range = FrameRange::new(seg_start, chunk_len);
            self.boundaries.push(ChunkBoundary {
                start_time: self.sample_rate.seconds_from_frames(seg_start.to_count()),
                end_time: self.sample_rate.seconds_from_frames(chunk_end.to_count()),
                frame_range,
                source_segment_ids: vec![span.segment_idx],
            });
            seg_start = chunk_end;
            remaining = remaining.saturating_sub(chunk_len);
        }
        self.reset_to(seg_start);
        true
    }

    fn attach_span(&mut self, span: Span) {
        if self.current_segments.is_empty() {
            self.current_frame_start = Some(span.frame_range.start);
        }
        if self.current_segments.last().copied() != Some(span.segment_idx) {
            self.current_segments.push(span.segment_idx);
        }
        self.current_frame_end = span.frame_range.end();
    }

    fn reset_to(&mut self, start: FrameIndex) {
        self.current_frame_start = Some(start);
        self.current_frame_end = start;
        self.current_segments.clear();
    }

    fn duration_frames(&self) -> FrameCount {
        self.current_frame_start
            .map(|start| self.current_frame_end.saturating_sub(start))
            .unwrap_or(FrameCount::ZERO)
    }
}

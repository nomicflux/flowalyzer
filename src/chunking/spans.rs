use crate::types::{FrameRange, SampleRate, Transcript};

pub(super) const EPS: f64 = 1e-9;

#[derive(Clone, Copy)]
pub(super) struct Span {
    pub(super) segment_idx: usize,
    pub(super) start_time: f64,
    pub(super) end_time: f64,
    pub(super) frame_range: FrameRange,
}

impl Span {
    pub(super) fn duration(self) -> f64 {
        self.end_time - self.start_time
    }

    pub(super) fn frame_length(self) -> usize {
        self.frame_range.length.as_usize()
    }
}

pub(super) fn build_spans(
    transcript: &Transcript,
    pauses: &[f64],
    sample_rate: SampleRate,
) -> Vec<Span> {
    if pauses.is_empty() {
        return transcript
            .segments
            .iter()
            .enumerate()
            .map(|(idx, segment)| {
                let frame_range =
                    FrameRange::from_times(segment.start_time, segment.end_time, sample_rate);
                Span {
                    segment_idx: idx,
                    start_time: segment.start_time,
                    end_time: segment.end_time,
                    frame_range,
                }
            })
            .collect();
    }

    let mut spans = Vec::new();
    let mut pause_idx = 0;

    for (idx, segment) in transcript.segments.iter().enumerate() {
        let mut span_start = segment.start_time;

        while pause_idx < pauses.len() && pauses[pause_idx] <= span_start + EPS {
            pause_idx += 1;
        }

        let mut iter_idx = pause_idx;
        while iter_idx < pauses.len() {
            let pause_time = pauses[iter_idx];
            if pause_time >= segment.end_time - EPS {
                break;
            }
            if pause_time > span_start + EPS {
                let frame_range = FrameRange::from_times(span_start, pause_time, sample_rate);
                if frame_range.length.as_usize() > 0 {
                    spans.push(Span {
                        segment_idx: idx,
                        start_time: span_start,
                        end_time: pause_time,
                        frame_range,
                    });
                }
                span_start = pause_time;
            }
            iter_idx += 1;
        }

        if segment.end_time - span_start > EPS {
            let frame_range = FrameRange::from_times(span_start, segment.end_time, sample_rate);
            if frame_range.length.as_usize() > 0 {
                spans.push(Span {
                    segment_idx: idx,
                    start_time: span_start,
                    end_time: segment.end_time,
                    frame_range,
                });
            }
        }

        pause_idx = iter_idx;
    }

    spans
}

use crate::types::Transcript;

pub(super) const EPS: f64 = 1e-9;

#[derive(Clone, Copy)]
pub(super) struct Span {
    pub(super) segment_idx: usize,
    pub(super) start_time: f64,
    pub(super) end_time: f64,
}

impl Span {
    pub(super) fn duration(self) -> f64 {
        self.end_time - self.start_time
    }
}

pub(super) fn build_spans(transcript: &Transcript, pauses: &[f64]) -> Vec<Span> {
    if pauses.is_empty() {
        return transcript
            .segments
            .iter()
            .enumerate()
            .map(|(idx, segment)| Span {
                segment_idx: idx,
                start_time: segment.start_time,
                end_time: segment.end_time,
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
                spans.push(Span {
                    segment_idx: idx,
                    start_time: span_start,
                    end_time: pause_time,
                });
                span_start = pause_time;
            }
            iter_idx += 1;
        }

        if segment.end_time - span_start > EPS {
            spans.push(Span {
                segment_idx: idx,
                start_time: span_start,
                end_time: segment.end_time,
            });
        }

        pause_idx = iter_idx;
    }

    spans
}

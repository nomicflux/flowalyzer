use std::time::Duration;

use ndarray::Array1;

use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, PronunciationError, PronunciationFeatures, Result,
};

/// Audio-only alignment placeholder that compares coarse spectral statistics.
#[derive(Debug, Default)]
pub struct AudioAligner;

impl AudioAligner {
    pub fn new() -> Self {
        Self
    }

    pub fn align(
        &self,
        reference: &PronunciationFeatures,
        learner: &PronunciationFeatures,
    ) -> Result<AlignmentReport> {
        ensure_features(reference, learner)?;
        let segment_frames = SEGMENT_FRAMES.min(reference.frame_count.max(1));
        let total_frames = reference.frame_count.min(learner.frame_count);

        let mut phonemes = Vec::new();
        let mut running_similarity = 0.0;
        let mut running_articulation = 0.0;
        let mut running_timing_delta = 0.0;

        let ref_energy = slice(&reference.energy)?;
        let learn_energy = slice(&learner.energy)?;
        let ref_flux = slice(&reference.spectral_flux)?;
        let learn_flux = slice(&learner.spectral_flux)?;

        let mut frame = 0;
        while frame < total_frames {
            let start = frame;
            let end = (frame + segment_frames).min(total_frames);
            let stats = SegmentStats::compute(
                reference,
                learner,
                ref_energy,
                learn_energy,
                ref_flux,
                learn_flux,
                start,
                end,
            );

            let reference_start_ms = frame_to_ms(start);
            let reference_end_ms = frame_to_ms(end);
            let learner_start_ms = (reference_start_ms + stats.timing_delta_ms).max(0.0);
            let learner_end_ms = (reference_end_ms + stats.timing_delta_ms).max(0.0);

            phonemes.push(AlignedPhoneme {
                symbol: format!("#{}", phonemes.len() + 1),
                reference_start_ms,
                reference_end_ms,
                learner_start_ms,
                learner_end_ms,
                timing_delta_ms: stats.timing_delta_ms,
                similarity: stats.similarity,
                articulation_variance: stats.articulation_variance,
            });

            running_similarity += stats.similarity;
            running_articulation += stats.articulation_variance;
            running_timing_delta += stats.timing_delta_ms;
            frame = end;
        }

        let duration_ms = frame_to_ms(total_frames).max(0.0);
        let segment_count = phonemes.len().max(1) as f32;
        let confidence = (running_similarity / segment_count).clamp(0.0, 1.0);

        Ok(AlignmentReport {
            phonemes,
            total_duration: Duration::from_millis(duration_ms.round() as u64),
            reference_path_cost: running_articulation,
            learner_path_cost: running_articulation,
            global_time_offset_ms: running_timing_delta / segment_count,
            confidence,
        })
    }
}

const FRAME_HOP_MS: f32 = 10.0;
const SEGMENT_FRAMES: usize = 12;

fn ensure_features(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
) -> Result<()> {
    if reference.frame_count == 0 {
        return Err(PronunciationError::new(
            "reference features contain no frames for alignment",
        ));
    }
    if learner.frame_count == 0 {
        return Err(PronunciationError::new(
            "learner features contain no frames for alignment",
        ));
    }
    Ok(())
}

fn frame_to_ms(frame: usize) -> f32 {
    frame as f32 * FRAME_HOP_MS
}

fn slice(array: &Array1<f32>) -> Result<&[f32]> {
    array.as_slice().ok_or_else(|| {
        PronunciationError::new("feature slice is not contiguous; cannot compute alignment stats")
    })
}

struct SegmentStats {
    similarity: f32,
    articulation_variance: f32,
    timing_delta_ms: f32,
}

impl SegmentStats {
    #[allow(clippy::too_many_arguments)]
    fn compute(
        reference: &PronunciationFeatures,
        learner: &PronunciationFeatures,
        ref_energy: &[f32],
        learn_energy: &[f32],
        ref_flux: &[f32],
        learn_flux: &[f32],
        start: usize,
        end: usize,
    ) -> Self {
        let mfcc_similarity =
            average_mfcc_similarity(reference, learner, start, end).unwrap_or(0.0);
        let flux_variance = average_abs_difference(ref_flux, learn_flux, start, end);
        let timing_delta_ms =
            peak_timing_delta(ref_energy, learn_energy, start, end) * FRAME_HOP_MS;

        Self {
            similarity: mfcc_similarity,
            articulation_variance: flux_variance,
            timing_delta_ms,
        }
    }
}

fn average_mfcc_similarity(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
    start: usize,
    end: usize,
) -> Option<f32> {
    let frame_span = end.checked_sub(start)?;
    if frame_span == 0 {
        return None;
    }

    let mut total_distance = 0.0;
    for frame in start..end {
        let ref_row = reference.mfcc.row(frame);
        let learn_row = learner.mfcc.row(frame);
        let coeffs = ref_row.len().min(learn_row.len());
        if coeffs == 0 {
            continue;
        }
        let distance: f32 = ref_row
            .iter()
            .zip(learn_row.iter())
            .take(coeffs)
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / coeffs as f32;
        total_distance += distance;
    }

    let mean_distance = total_distance / frame_span as f32;
    Some((1.0 / (1.0 + mean_distance)).clamp(0.0, 1.0))
}

fn average_abs_difference(data_a: &[f32], data_b: &[f32], start: usize, end: usize) -> f32 {
    let span = end.saturating_sub(start);
    if span == 0 {
        return 0.0;
    }
    let mut total = 0.0;
    for idx in start..end {
        total += (data_a.get(idx).copied().unwrap_or_default()
            - data_b.get(idx).copied().unwrap_or_default())
        .abs();
    }
    (total / span as f32).min(1.0)
}

fn peak_timing_delta(ref_energy: &[f32], learn_energy: &[f32], start: usize, end: usize) -> f32 {
    let span = end.saturating_sub(start);
    if span == 0 {
        return 0.0;
    }
    let ref_peak = peak_index(ref_energy, start, end);
    let learner_peak = peak_index(learn_energy, start, end);
    (learner_peak as i32 - ref_peak as i32) as f32
}

fn peak_index(data: &[f32], start: usize, end: usize) -> usize {
    let mut best_idx = start;
    let mut best_value = f32::MIN;
    for idx in start..end {
        let value = data.get(idx).copied().unwrap_or(0.0);
        if value > best_value {
            best_value = value;
            best_idx = idx;
        }
    }
    best_idx
}

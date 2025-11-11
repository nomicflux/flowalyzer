use crate::pronunciation::{PronunciationError, PronunciationFeatures, Result};

use super::templates::PhonemeTemplate;

const FRAME_HOP_MS: f32 = 10.0;

/// Alignment outcome produced by the DTW solver.
#[derive(Debug, Clone)]
pub struct DtwAlignment {
    pub segments: Vec<AlignmentSegment>,
    pub total_cost: f32,
}

/// Per-phoneme timing information derived from warping paths.
#[derive(Debug, Clone)]
pub struct AlignmentSegment {
    pub symbol: String,
    pub learner_start_frame: usize,
    pub learner_end_frame: usize,
    pub reference_start_frame: usize,
    pub reference_end_frame: usize,
    pub cost: f32,
    pub similarity: f32,
}

/// Executes a monotonic dynamic time warping between reference templates and learner frames.
pub fn align_templates(
    templates: &[PhonemeTemplate],
    learner: &PronunciationFeatures,
) -> Result<DtwAlignment> {
    ensure_inputs(templates, learner.frame_count)?;
    let mut dp = vec![vec![f32::INFINITY; learner.frame_count + 1]; templates.len() + 1];
    let mut backtrack = vec![vec![usize::MAX; learner.frame_count + 1]; templates.len() + 1];
    dp[0][0] = 0.0;
    fill_tables(&mut dp, &mut backtrack, templates, learner)?;
    let total_cost = dp[templates.len()][learner.frame_count];
    if !total_cost.is_finite() {
        return Err(PronunciationError::new(
            "failed to compute finite DTW alignment cost",
        ));
    }
    let segments = backtrack_segments(&backtrack, templates, learner)?;
    Ok(DtwAlignment {
        segments,
        total_cost,
    })
}

fn ensure_inputs(templates: &[PhonemeTemplate], frame_count: usize) -> Result<()> {
    if templates.is_empty() {
        return Err(PronunciationError::new(
            "template sequence must contain at least one phoneme",
        ));
    }
    if frame_count < templates.len() {
        return Err(PronunciationError::new(format!(
            "learner features contain {frame_count} frames, \
             fewer than the {} templates provided",
            templates.len()
        )));
    }
    Ok(())
}

fn fill_tables(
    dp: &mut [Vec<f32>],
    backtrack: &mut [Vec<usize>],
    templates: &[PhonemeTemplate],
    learner: &PronunciationFeatures,
) -> Result<()> {
    for i in 1..=templates.len() {
        for j in i..=learner.frame_count {
            update_cell(dp, backtrack, templates, learner, i, j)?;
        }
    }
    Ok(())
}

fn update_cell(
    dp: &mut [Vec<f32>],
    backtrack: &mut [Vec<usize>],
    templates: &[PhonemeTemplate],
    learner: &PronunciationFeatures,
    i: usize,
    j: usize,
) -> Result<()> {
    let template = &templates[i - 1];
    let mut best_cost = f32::INFINITY;
    let mut best_k = usize::MAX;
    for k in (i - 1)..j {
        let previous = dp[i - 1][k];
        if !previous.is_finite() {
            continue;
        }
        let segment_cost = segment_cost(template, learner, k, j)?;
        let candidate = previous + segment_cost;
        if candidate < best_cost {
            best_cost = candidate;
            best_k = k;
        }
    }
    dp[i][j] = best_cost;
    backtrack[i][j] = best_k;
    Ok(())
}

fn backtrack_segments(
    backtrack: &[Vec<usize>],
    templates: &[PhonemeTemplate],
    learner: &PronunciationFeatures,
) -> Result<Vec<AlignmentSegment>> {
    let mut segments = Vec::with_capacity(templates.len());
    let mut end = learner.frame_count;
    for i in (1..=templates.len()).rev() {
        let start = backtrack[i][end];
        if start == usize::MAX || start >= end {
            return Err(PronunciationError::new("invalid DTW backtrack encountered"));
        }
        let template = &templates[i - 1];
        let cost = segment_cost(template, learner, start, end)?;
        segments.push(AlignmentSegment {
            symbol: template.symbol.clone(),
            learner_start_frame: start,
            learner_end_frame: end,
            reference_start_frame: template.start_frame,
            reference_end_frame: template.end_frame,
            cost,
            similarity: similarity_from_cost(cost),
        });
        end = start;
    }
    segments.reverse();
    Ok(segments)
}

fn segment_cost(
    template: &PhonemeTemplate,
    features: &PronunciationFeatures,
    start: usize,
    end: usize,
) -> Result<f32> {
    if start >= end {
        return Err(PronunciationError::new(
            "segment cost requires a non-empty frame range",
        ));
    }
    let frame_span = (end - start) as f32;
    let mfcc_slice = features.mfcc.slice(ndarray::s![start..end, ..]);
    let mut distance = 0.0;
    for row in mfcc_slice.rows() {
        distance += row
            .iter()
            .zip(template.centroid.iter())
            .map(|(a, b)| {
                let diff = *a - *b;
                diff * diff
            })
            .sum::<f32>();
    }
    let average_distance = (distance / frame_span).sqrt();

    let energy_slice = features.energy.slice(ndarray::s![start..end]);
    let average_energy = energy_slice.iter().copied().sum::<f32>() / frame_span;
    let energy_penalty = (average_energy - template.average_energy).abs();

    Ok(average_distance + energy_penalty)
}

fn similarity_from_cost(cost: f32) -> f32 {
    1.0 / (1.0 + cost.max(0.0))
}

/// Converts frame indices to millisecond offsets using the configured hop size.
pub fn frames_to_ms(start: usize, end: usize) -> (f32, f32) {
    let start_ms = start as f32 * FRAME_HOP_MS;
    let end_ms = end as f32 * FRAME_HOP_MS;
    (start_ms, end_ms)
}

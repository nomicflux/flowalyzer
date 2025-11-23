use std::time::Duration;

use crate::pronunciation::features::{ChunkFeatures, ReferenceFeatures};
use crate::pronunciation::session::AlignmentReport;

const FRAME_HOP_MS: u64 = 10;

#[derive(Debug, Default, Clone, Copy)]
pub struct StatelessAligner;

impl StatelessAligner {
    pub fn new() -> Self {
        Self
    }

    pub fn align(
        &self,
        reference: &ReferenceFeatures,
        learner: &ChunkFeatures,
        global_offset_ms: f32,
    ) -> AlignmentReport {
        align_features(reference, learner, global_offset_ms)
    }
}

pub fn align_features(
    reference: &ReferenceFeatures,
    learner: &ChunkFeatures,
    global_offset_ms: f32,
) -> AlignmentReport {
    assert_eq!(
        learner.energy.len(),
        learner.pitch.len(),
        "learner frames must align"
    );
    assert!(
        reference.energy.len() >= learner.energy.len(),
        "reference energy frames exhausted ({} available, {} required)",
        reference.energy.len(),
        learner.energy.len()
    );
    assert!(
        reference.pitch.len() >= learner.pitch.len(),
        "reference pitch frames exhausted ({} available, {} required)",
        reference.pitch.len(),
        learner.pitch.len()
    );
    assert!(
        reference.frame_starts.len() >= learner.frame_starts.len(),
        "reference frame indices exhausted"
    );
    for (ref_idx, learner_idx) in reference
        .frame_starts
        .iter()
        .zip(learner.frame_starts.iter())
        .take(learner.frame_starts.len())
    {
        assert_eq!(
            ref_idx, learner_idx,
            "frame index mismatch between reference and learner"
        );
    }

    let similarity = compute_similarity(&reference.energy, &learner.energy);
    let contour = compute_contour_similarity(&reference.pitch, &learner.pitch);
    let confidence = compute_confidence(&similarity, &contour);

    AlignmentReport {
        reference_energy: reference.energy.clone(),
        learner_energy: learner.energy.clone(),
        reference_pitch: reference.pitch.clone(),
        learner_pitch: learner.pitch.clone(),
        similarity_band: similarity,
        contour_band: contour,
        phonemes: Vec::new(),
        total_duration: Duration::from_millis((learner.energy.len() as u64) * FRAME_HOP_MS),
        global_time_offset_ms: global_offset_ms,
        confidence,
    }
}

fn compute_similarity(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    assert_eq!(reference.len(), learner.len(), "frames must align");
    (0..reference.len())
        .map(|index| frame_similarity(reference[index], learner[index]))
        .collect()
}

fn frame_similarity(reference_value: f32, learner_value: f32) -> f32 {
    let diff = (reference_value - learner_value).abs();
    let max_value = reference_value
        .abs()
        .max(learner_value.abs())
        .max(f32::MIN_POSITIVE);
    1.0 - diff / max_value
}

fn compute_contour_similarity(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    assert_eq!(reference.len(), learner.len(), "frames must align");
    if learner.is_empty() {
        return Vec::new();
    }
    (0..learner.len())
        .map(|i| pitch_frame_similarity(reference[i], learner[i]))
        .collect()
}

fn pitch_frame_similarity(reference_pitch: f32, learner_pitch: f32) -> f32 {
    assert!(
        reference_pitch.is_finite() && learner_pitch.is_finite(),
        "pitch values must be finite"
    );
    let ref_p = reference_pitch.abs().max(f32::MIN_POSITIVE);
    let learner_p = learner_pitch.abs().max(f32::MIN_POSITIVE);
    let semitone_diff = (ref_p / learner_p).log2().abs() * 12.0;
    1.0 - semitone_diff / 12.0
}

fn compute_confidence(similarity: &[f32], contour: &[f32]) -> f32 {
    let similarity_avg = average(similarity);
    let contour_avg = average(contour);
    if similarity.is_empty() && contour.is_empty() {
        0.0
    } else {
        (similarity_avg + contour_avg) / 2.0
    }
}

fn average(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

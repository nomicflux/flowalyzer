use std::time::Duration;

use crate::pronunciation::session::AlignmentReport;

const FRAME_HOP_MS: u64 = 10;

pub fn align_chunk(
    reference_energy: &[f32],
    learner_energy: &[f32],
    reference_pitch: &[f32],
    learner_pitch: &[f32],
    global_offset_ms: f32,
) -> AlignmentReport {
    let similarity = compute_similarity(reference_energy, learner_energy);
    let contour = compute_contour_similarity(reference_pitch, learner_pitch);
    let confidence = compute_confidence(&similarity, &contour);

    AlignmentReport {
        reference_energy: reference_energy.to_vec(),
        learner_energy: learner_energy.to_vec(),
        reference_pitch: reference_pitch.to_vec(),
        learner_pitch: learner_pitch.to_vec(),
        similarity_band: similarity,
        contour_band: contour,
        phonemes: Vec::new(),
        total_duration: Duration::from_millis((learner_energy.len() as u64) * FRAME_HOP_MS),
        global_time_offset_ms: global_offset_ms,
        confidence,
    }
}

fn compute_similarity(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    let len = reference.len().min(learner.len());
    (0..len)
        .map(|index| frame_similarity(reference[index], learner[index]))
        .collect()
}

fn frame_similarity(reference_value: f32, learner_value: f32) -> f32 {
    let diff = (reference_value - learner_value).abs();
    let max_value = reference_value.abs().max(learner_value.abs()).max(1e-3);
    (1.0 - diff / max_value).clamp(0.0, 1.0)
}

fn compute_contour_similarity(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    let len = reference.len().min(learner.len());
    (0..len)
        .map(|index| pitch_similarity(reference[index], learner[index]))
        .collect()
}

fn pitch_similarity(reference_value: f32, learner_value: f32) -> f32 {
    if reference_value == 0.0 && learner_value == 0.0 {
        return 1.0;
    }
    if reference_value == 0.0 || learner_value == 0.0 {
        return 0.0;
    }
    let ratio = (reference_value / learner_value).abs();
    (ratio.min(1.0 / ratio)).clamp(0.0, 1.0)
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

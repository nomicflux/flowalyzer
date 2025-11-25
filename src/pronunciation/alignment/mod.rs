use crate::pronunciation::features::{ChunkFeatures, ReferenceFeatures};
use crate::pronunciation::session::AlignmentReport;

#[derive(Debug, Clone, Copy)]
pub struct StatelessAligner;

impl StatelessAligner {
    pub fn new() -> Self {
        Self
    }

    pub fn align(
        &self,
        reference: &ReferenceFeatures,
        learner: &ChunkFeatures,
        start_frame_idx: usize,
        global_offset_ms: f32,
        sample_rate: u32,
        hop_samples: usize,
    ) -> AlignmentReport {
        align_features(
            reference,
            learner,
            start_frame_idx,
            global_offset_ms,
            sample_rate,
            hop_samples,
        )
    }
}

impl Default for StatelessAligner {
    fn default() -> Self {
        Self::new()
    }
}

pub fn align_features(
    reference: &ReferenceFeatures,
    learner: &ChunkFeatures,
    start_frame_idx: usize,
    global_offset_ms: f32,
    sample_rate: u32,
    hop_samples: usize,
) -> AlignmentReport {
    let end_frame_idx = start_frame_idx + learner.energy.len();
    let reference_energy = reference.energy[start_frame_idx..end_frame_idx].to_vec();
    let reference_pitch = reference.pitch[start_frame_idx..end_frame_idx].to_vec();

    let energy_error = compute_energy_error(&reference_energy, &learner.energy);
    let similarity = compute_similarity(
        &reference_energy,
        &learner.energy,
        &reference_pitch,
        &learner.pitch,
    );
    let contour = compute_contour_band(&reference_pitch, &learner.pitch);
    let hop_ms = hop_samples as f32 / sample_rate as f32 * 1000.0;
    AlignmentReport {
        reference_energy,
        learner_energy: learner.energy.clone(),
        reference_pitch,
        learner_pitch: learner.pitch.clone(),
        energy_error,
        similarity_band: similarity,
        contour_band: contour,
        start_frame_idx,
        end_frame_idx,
        hop_ms,
        global_time_offset_ms: global_offset_ms,
        total_duration: hop_ms * learner.energy.len() as f32,
    }
}

fn compute_energy_error(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    (0..reference.len())
        .map(|index| learner[index] - reference[index])
        .collect()
}

fn compute_similarity(
    reference_energy: &[f32],
    learner_energy: &[f32],
    reference_pitch: &[f32],
    learner_pitch: &[f32],
) -> Vec<f32> {
    (0..reference_energy.len())
        .map(|index| {
            let energy_ratio = (learner_energy[index] / reference_energy[index]).ln().abs();
            let pitch_ratio = (learner_pitch[index] / reference_pitch[index]).ln().abs();
            -(energy_ratio + pitch_ratio)
        })
        .collect()
}

fn compute_contour_band(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    (0..learner.len())
        .map(|i| 1200.0 * (learner[i] / reference[i]).log2())
        .collect()
}

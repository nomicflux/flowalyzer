use crate::pronunciation::features::{ChunkFeatures, ReferenceFeatures};
use crate::pronunciation::session::AlignmentReport;

pub fn align_features(
    reference: &ReferenceFeatures,
    learner: &ChunkFeatures,
    start_frame_idx: usize,
    global_offset_ms: f32,
    sample_rate: u32,
    hop_samples: usize,
) -> AlignmentReport {
    let ref_len = reference.energy.len().min(reference.pitch.len());
    let effective_start = start_frame_idx.min(ref_len);
    let available_ref = ref_len.saturating_sub(effective_start);
    let learner_len = learner
        .energy
        .len()
        .min(learner.pitch.len())
        .min(available_ref);

    if learner_len == 0 {
        let hop_ms = hop_samples as f32 / sample_rate as f32 * 1000.0;
        return AlignmentReport {
            reference_energy: Vec::new(),
            learner_energy: Vec::new(),
            energy_error: Vec::new(),
            reference_pitch: Vec::new(),
            learner_pitch: Vec::new(),
            similarity_band: Vec::new(),
            contour_band: Vec::new(),
            start_frame_idx: effective_start,
            end_frame_idx: effective_start,
            hop_ms,
            global_time_offset_ms: global_offset_ms,
            total_duration: 0.0,
        };
    }

    let end_frame_idx = effective_start + learner_len;
    let reference_energy = reference.energy[effective_start..end_frame_idx].to_vec();
    let reference_pitch = reference.pitch[effective_start..end_frame_idx].to_vec();
    let learner_energy = learner.energy[..learner_len].to_vec();
    let learner_pitch = learner.pitch[..learner_len].to_vec();

    let energy_error = compute_energy_error(&reference_energy, &learner_energy);
    let similarity =
        compute_similarity(&reference_energy, &learner_energy, &reference_pitch, &learner_pitch);
    let contour = compute_contour_band(&reference_pitch, &learner_pitch);
    let hop_ms = hop_samples as f32 / sample_rate as f32 * 1000.0;
    AlignmentReport {
        reference_energy,
        learner_energy,
        reference_pitch,
        learner_pitch,
        energy_error,
        similarity_band: similarity,
        contour_band: contour,
        start_frame_idx: effective_start,
        end_frame_idx,
        hop_ms,
        global_time_offset_ms: global_offset_ms,
        total_duration: hop_ms * learner_len as f32,
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
    let learner_log_pitch: Vec<f32> = learner_pitch.iter().map(|p| (1.0 + p).ln()).collect();
    let reference_log_pitch: Vec<f32> = reference_pitch.iter().map(|p| (1.0 + p).ln()).collect();
    let pitch_offset = mean(&learner_log_pitch) - mean(&reference_log_pitch);

    (0..reference_energy.len())
        .map(|index| {
            let energy_delta = (learner_energy[index] - reference_energy[index]).abs();
            let pitch_delta =
                (learner_log_pitch[index] - pitch_offset - reference_log_pitch[index]).abs();
            -(energy_delta + pitch_delta)
        })
        .collect()
}

fn compute_contour_band(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    let learner_log_pitch: Vec<f32> = learner.iter().map(|p| (1.0 + p).ln()).collect();
    let reference_log_pitch: Vec<f32> = reference.iter().map(|p| (1.0 + p).ln()).collect();
    let pitch_offset = mean(&learner_log_pitch) - mean(&reference_log_pitch);

    (0..learner.len())
        .map(|i| learner_log_pitch[i] - pitch_offset - reference_log_pitch[i])
        .collect()
}

fn mean(values: &[f32]) -> f32 {
    values.iter().sum::<f32>() / values.len() as f32
}

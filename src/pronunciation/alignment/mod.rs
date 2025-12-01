use crate::pronunciation::features::{ChunkFeatures, ReferenceFeatures};
use crate::pronunciation::session::AlignmentReport;

const EPS: f32 = 1e-6;

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
    let ref_log_energy: Vec<f32> = reference_energy
        .iter()
        .map(|e| (e.abs() + EPS).ln())
        .collect();
    let energy_scale = robust_spread(&ref_log_energy).max(EPS);

    let voiced_ref_log_pitch: Vec<f32> = reference_pitch
        .iter()
        .filter(|p| **p > 0.0)
        .map(|p| (p + EPS).ln())
        .collect();
    let pitch_scale = if voiced_ref_log_pitch.is_empty() {
        1.0
    } else {
        robust_spread(&voiced_ref_log_pitch).max(EPS)
    };

    (0..reference_energy.len())
        .map(|index| {
            let ref_energy = reference_energy[index];
            let learn_energy = learner_energy[index];
            let ref_pitch = reference_pitch[index];
            let learn_pitch = learner_pitch[index];

            let ref_silent = ref_energy.abs() <= EPS;
            let learn_silent = learn_energy.abs() <= EPS;

            if ref_silent && learn_silent {
                return 1.0;
            }
            if ref_silent ^ learn_silent {
                return -1.0;
            }

            let energy_ratio = (learn_energy.abs() + EPS) / (ref_energy.abs() + EPS);
            let energy_delta = energy_ratio.ln().abs();
            let energy_norm = energy_delta / energy_scale;

            let pitch_norm = if ref_pitch <= 0.0 && learn_pitch <= 0.0 {
                0.0
            } else if ref_pitch <= 0.0 || learn_pitch <= 0.0 {
                // Unvoiced mismatch: penalize relative to scale
                1.0 / pitch_scale
            } else {
                let pitch_ratio = (learn_pitch + EPS) / (ref_pitch + EPS);
                let semitone_delta = 12.0 * (pitch_ratio.log2()).abs();
                semitone_delta / pitch_scale
            };

            1.0 - (energy_norm + pitch_norm)
        })
        .collect()
}

fn compute_contour_band(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    let learner_log_pitch: Vec<f32> = learner.iter().map(|p| (1.0 + p).ln()).collect();
    let reference_log_pitch: Vec<f32> = reference.iter().map(|p| (1.0 + p).ln()).collect();
    let voiced: Vec<usize> = learner
        .iter()
        .zip(reference.iter())
        .enumerate()
        .filter_map(|(idx, (l, r))| if *l > 0.0 && *r > 0.0 { Some(idx) } else { None })
        .collect();
    if voiced.is_empty() {
        return vec![0.0; learner.len()];
    }

    let voiced_offset =
        mean_masked(&learner_log_pitch, &voiced) - mean_masked(&reference_log_pitch, &voiced);
    let mut smoothed = vec![0.0; learner.len()];

    // Median smoothing over a 3-frame window on voiced frames; leave unvoiced as zero.
    for i in 0..learner.len() {
        if learner[i] <= 0.0 || reference[i] <= 0.0 {
            smoothed[i] = 0.0;
            continue;
        }
        let start = i.saturating_sub(1);
        let end = (i + 2).min(learner.len());
        let mut window: Vec<f32> = (start..end)
            .filter_map(|j| {
                if learner[j] > 0.0 && reference[j] > 0.0 {
                    Some(learner_log_pitch[j] - voiced_offset - reference_log_pitch[j])
                } else {
                    None
                }
            })
            .collect();
        if window.is_empty() {
            smoothed[i] = 0.0;
        } else {
            window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = window.len() / 2;
            let median = if window.len().is_multiple_of(2) {
                // For even windows, prefer the lower median to preserve sign and avoid cancellation.
                window[mid - 1]
            } else {
                window[mid]
            };
            smoothed[i] = median;
        }
    }

    smoothed
}

fn mean_masked(values: &[f32], indices: &[usize]) -> f32 {
    let sum: f32 = indices.iter().map(|&i| values[i]).sum();
    sum / indices.len() as f32
}

fn robust_spread(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 1.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p10 = percentile(&sorted, 0.10);
    let p90 = percentile(&sorted, 0.90);
    let spread = (p90 - p10).abs();
    if spread < EPS {
        1.0
    } else {
        spread
    }
}

fn percentile(sorted: &[f32], pct: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let clamped = pct.clamp(0.0, 1.0);
    let idx = ((sorted.len() - 1) as f32 * clamped).round() as usize;
    sorted[idx]
}

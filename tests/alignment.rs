use flowalyzer::pronunciation::alignment::align_features;
use flowalyzer::pronunciation::features::{ChunkFeatures, ReferenceFeatures};

fn reference_features(energy: Vec<f32>, pitch: Vec<f32>) -> ReferenceFeatures {
    let aligned_len = energy.len().min(pitch.len());
    ReferenceFeatures {
        energy: energy.into_iter().take(aligned_len).collect(),
        pitch: pitch.into_iter().take(aligned_len).collect(),
        frame_starts: (0..aligned_len).collect(),
    }
}

fn chunk_features(energy: Vec<f32>, pitch: Vec<f32>) -> ChunkFeatures {
    let aligned_len = energy.len().min(pitch.len());
    ChunkFeatures {
        energy: energy.into_iter().take(aligned_len).collect(),
        pitch: pitch.into_iter().take(aligned_len).collect(),
        frame_starts: (0..aligned_len).collect(),
    }
}

#[test]
fn identical_chunks_produce_high_similarity() {
    let energy = vec![0.5, 0.6, 0.55];
    let pitch = vec![220.0, 225.0, 230.0];
    let reference = reference_features(energy.clone(), pitch.clone());
    let learner = chunk_features(energy, pitch);
    let report = align_features(&reference, &learner, 0.0);
    assert!(report.similarity_band.iter().all(|value| *value > 0.99));
    assert!(report.contour_band.iter().all(|value| *value > 0.99));
    assert!(report.confidence > 0.99);
}

#[test]
fn mismatched_chunks_reduce_confidence() {
    let reference_energy = vec![0.5, 0.6, 0.55];
    let learner_energy = vec![0.2, 0.1, 0.3];
    let reference_pitch = vec![220.0, 225.0, 230.0];
    let learner_pitch = vec![110.0, 115.0, 120.0];
    let reference = reference_features(reference_energy, reference_pitch);
    let learner = chunk_features(learner_energy, learner_pitch);
    let report = align_features(&reference, &learner, 120.0);
    assert!(report.confidence < 0.8);
    assert_eq!(report.global_time_offset_ms, 120.0);
}

#[test]
fn contour_penalizes_unvoiced_frames() {
    let reference = reference_features(vec![0.5, 0.6, 0.55], vec![220.0, 0.0, 230.0]);
    let learner = chunk_features(vec![0.5, 0.6, 0.55], vec![220.0, 225.0, 0.0]);
    let report = align_features(&reference, &learner, 0.0);
    assert_eq!(report.contour_band.len(), 3);
    // Middle frame should be penalized below voiced matches due to unvoiced pitch on one side.
    assert!(report.contour_band[1] < report.contour_band[0]);
}

#[test]
fn contour_drops_to_zero_for_octave_difference() {
    let reference = reference_features(vec![0.5], vec![220.0]);
    let learner = chunk_features(vec![0.5], vec![440.0]);
    let report = align_features(&reference, &learner, 0.0);
    assert_eq!(report.contour_band.len(), 1);
    assert_eq!(report.contour_band[0], 0.0);
}

#[test]
fn contour_treats_mutual_silence_as_match() {
    let reference = reference_features(vec![0.0, 0.0], vec![0.0, 0.0]);
    let learner = chunk_features(vec![0.0, 0.0], vec![0.0, 0.0]);
    let report = align_features(&reference, &learner, 0.0);
    assert_eq!(report.contour_band.len(), 2);
    assert!(report.contour_band.iter().all(|v| (*v - 1.0).abs() < 1e-6));
}

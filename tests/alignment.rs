use flowalyzer::pronunciation::alignment::align_features;
use flowalyzer::pronunciation::features::FeatureFrames;

#[test]
fn identical_chunks_produce_high_similarity() {
    let energy = vec![0.5, 0.6, 0.55];
    let pitch = vec![220.0, 225.0, 230.0];
    let reference = FeatureFrames {
        energy: energy.clone(),
        pitch: pitch.clone(),
    };
    let learner = FeatureFrames { energy, pitch };
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
    let reference = FeatureFrames {
        energy: reference_energy,
        pitch: reference_pitch,
    };
    let learner = FeatureFrames {
        energy: learner_energy,
        pitch: learner_pitch,
    };
    let report = align_features(&reference, &learner, 120.0);
    assert!(report.confidence < 0.8);
    assert_eq!(report.global_time_offset_ms, 120.0);
}

#[test]
fn contour_penalizes_unvoiced_frames() {
    let reference = FeatureFrames {
        energy: vec![0.5, 0.6, 0.55],
        pitch: vec![220.0, 0.0, 230.0],
    };
    let learner = FeatureFrames {
        energy: vec![0.5, 0.6, 0.55],
        pitch: vec![220.0, 225.0, 0.0],
    };
    let report = align_features(&reference, &learner, 0.0);
    assert_eq!(report.contour_band.len(), 3);
    // Middle frame should be penalized to 0 due to unvoiced pitch on one side.
    assert_eq!(report.contour_band[1], 0.0);
}

#[test]
fn contour_drops_to_zero_for_octave_difference() {
    let reference = FeatureFrames {
        energy: vec![0.5],
        pitch: vec![220.0],
    };
    let learner = FeatureFrames {
        energy: vec![0.5],
        pitch: vec![440.0],
    };
    let report = align_features(&reference, &learner, 0.0);
    assert_eq!(report.contour_band.len(), 1);
    assert_eq!(report.contour_band[0], 0.0);
}

#[test]
fn contour_treats_mutual_silence_as_match() {
    let reference = FeatureFrames {
        energy: vec![0.0, 0.0],
        pitch: vec![0.0, 0.0],
    };
    let learner = FeatureFrames {
        energy: vec![0.0, 0.0],
        pitch: vec![0.0, 0.0],
    };
    let report = align_features(&reference, &learner, 0.0);
    assert_eq!(report.contour_band.len(), 2);
    assert!(report.contour_band.iter().all(|v| (*v - 1.0).abs() < 1e-6));
}

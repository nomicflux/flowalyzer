use flowalyzer::pronunciation::alignment::align_features;
use flowalyzer::pronunciation::features::{ChunkFeatures, ReferenceFeatures};

const SAMPLE_RATE: u32 = 16_000;
const HOP_SAMPLES: usize = 160;

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
        frame_starts: (0..aligned_len as isize).collect(),
    }
}

#[test]
fn similarity_handles_silence_with_silence() {
    let energy = vec![0.0, 0.0, 0.0];
    let pitch = vec![0.0, 0.0, 0.0];
    let reference = reference_features(energy.clone(), pitch.clone());
    let learner = chunk_features(energy, pitch);
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    assert!(report.energy_error.iter().all(|value| value.abs() < 1e-6));
    assert!(report
        .similarity_band
        .iter()
        .all(|value| (*value).abs() < 1e-6));
    assert!(report.contour_band.iter().all(|value| value.abs() < 1e-6));
    assert!((report.total_duration - 30.0).abs() < 1e-6);
    assert_eq!(report.start_frame_idx, 0);
    assert_eq!(report.end_frame_idx, 3);
    assert!((report.hop_ms - 10.0).abs() < 1e-6);
}

#[test]
fn similarity_marks_silence_against_sound() {
    let reference = reference_features(vec![0.0, 0.0], vec![0.0, 0.0]);
    let learner = chunk_features(vec![1.0, 1.0], vec![100.0, 120.0]);
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    assert_eq!(report.energy_error, vec![1.0, 1.0]);
    assert!(report.similarity_band.iter().all(|v| *v < 0.0));
    assert!(report.contour_band.iter().any(|v| *v < 0.0));
}

#[test]
fn similarity_marks_sound_against_silence() {
    let reference = reference_features(vec![1.0, 2.0], vec![100.0, 110.0]);
    let learner = chunk_features(vec![0.0, 0.0], vec![0.0, 0.0]);
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    assert_eq!(report.energy_error, vec![-1.0, -2.0]);
    assert!(report.similarity_band.iter().all(|v| *v < 0.0));
    assert!(report.contour_band.iter().any(|v| *v > 0.0));
}

#[test]
fn similarity_matches_equal_sound() {
    let reference = reference_features(vec![1.5, 2.0], vec![200.0, 220.0]);
    let learner = chunk_features(vec![1.5, 2.0], vec![200.0, 220.0]);
    let report = align_features(&reference, &learner, 0, 25.0, SAMPLE_RATE, HOP_SAMPLES);
    assert_eq!(report.energy_error, vec![0.0, 0.0]);
    assert_eq!(report.similarity_band, vec![0.0, 0.0]);
    assert_eq!(report.contour_band, vec![0.0, 0.0]);
    assert_eq!(report.start_frame_idx, 0);
    assert_eq!(report.end_frame_idx, 2);
    assert!((report.total_duration - 20.0).abs() < 1e-6);
    assert_eq!(report.global_time_offset_ms, 25.0);
}

#[test]
fn similarity_detects_difference_between_sounds() {
    let reference = reference_features(vec![1.0, 1.5], vec![180.0, 200.0]);
    let learner = chunk_features(vec![0.5, 2.0], vec![150.0, 260.0]);
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    assert_eq!(report.energy_error, vec![-0.5, 0.5]);
    assert!(report.similarity_band.iter().all(|v| *v < 0.0));
    assert!(report.contour_band.iter().any(|v| v.abs() > 0.1));
}

#[test]
fn truncates_on_pitch_length_mismatch() {
    let reference = flowalyzer::pronunciation::features::ReferenceFeatures {
        energy: vec![1.0, 1.0],
        pitch: vec![100.0, 100.0],
        frame_starts: vec![0, 1],
    };
    let learner = flowalyzer::pronunciation::features::ChunkFeatures {
        energy: vec![1.0],
        pitch: vec![200.0, 200.0],
        frame_starts: vec![0, 1],
    };
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    assert_eq!(report.reference_energy.len(), 1);
    assert_eq!(report.reference_pitch.len(), 1);
    assert_eq!(report.learner_energy.len(), 1);
    assert_eq!(report.learner_pitch.len(), 1);
}

#[test]
fn gracefully_truncates_when_reference_exhausted() {
    // Reference shorter than learner slice starting near the end; should not panic.
    let reference = reference_features(vec![0.1, 0.2, 0.3, 0.4], vec![200.0, 210.0, 220.0, 230.0]);
    let learner = chunk_features(vec![0.5, 0.6, 0.7], vec![205.0, 215.0, 225.0]);
    let report = align_features(&reference, &learner, 3, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    assert_eq!(
        report.start_frame_idx, 3,
        "should keep requested start within reference length"
    );
    assert_eq!(
        report.end_frame_idx, 4,
        "end_frame_idx should cap at reference length"
    );
    assert_eq!(
        report.reference_energy.len(),
        1,
        "only remaining reference frames should be used"
    );
    assert_eq!(
        report.learner_energy.len(),
        1,
        "learner frames should be truncated to match reference availability"
    );
    assert_eq!(
        report.total_duration,
        report.hop_ms * report.reference_energy.len() as f32
    );
    assert!(
        report
            .similarity_band
            .iter()
            .all(|v| v.is_finite()),
        "similarity values should remain finite after truncation"
    );
}

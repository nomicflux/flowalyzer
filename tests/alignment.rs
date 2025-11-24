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
        frame_starts: (0..aligned_len).collect(),
    }
}

#[test]
fn identical_chunks_keep_similarity_high() {
    let energy = vec![0.5, 0.6, 0.55];
    let pitch = vec![220.0, 225.0, 230.0];
    let reference = reference_features(energy.clone(), pitch.clone());
    let learner = chunk_features(energy, pitch);
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    assert!(report.energy_error.iter().all(|value| value.abs() < 1e-6));
    assert!(report
        .similarity_band
        .iter()
        .all(|value| (*value - 1.0).abs() < 1e-6));
    assert!(report.contour_band.iter().all(|value| value.abs() < 1e-6));
    assert!((report.total_duration - 30.0).abs() < 1e-6);
    assert_eq!(report.start_frame_idx, 0);
    assert_eq!(report.end_frame_idx, 3);
    assert!((report.hop_ms - 10.0).abs() < 1e-6);
}

#[test]
fn slices_reference_from_start_frame_index() {
    let reference = reference_features(vec![1.0, 2.0, 3.0, 4.0], vec![100.0, 200.0, 300.0, 400.0]);
    let learner = chunk_features(vec![3.0, 4.0], vec![300.0, 400.0]);
    let report = align_features(&reference, &learner, 2, 15.0, SAMPLE_RATE, HOP_SAMPLES);
    assert_eq!(report.reference_energy, vec![3.0, 4.0]);
    assert_eq!(report.reference_pitch, vec![300.0, 400.0]);
    assert!(report
        .similarity_band
        .iter()
        .all(|value| (*value - 1.0).abs() < 1e-6));
    assert!(report.contour_band.iter().all(|value| value.abs() < 1e-6));
    assert_eq!(report.global_time_offset_ms, 15.0);
    assert!((report.hop_ms - 10.0).abs() < 1e-6);
    assert_eq!(report.start_frame_idx, 2);
    assert_eq!(report.end_frame_idx, 4);
    assert!((report.total_duration - 20.0).abs() < 1e-6);
}

#[test]
fn computes_raw_metric_bands() {
    let reference = reference_features(
        vec![1.0, 1.0, 1.0, 1.0, 1.0],
        vec![100.0, 100.0, 100.0, 100.0, 100.0],
    );
    let learner = chunk_features(
        vec![2.0, 2.0, 2.0, 2.0, 2.0],
        vec![200.0, 200.0, 200.0, 200.0, 200.0],
    );
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    assert_eq!(report.energy_error, vec![1.0; 5]);
    assert_eq!(report.similarity_band, vec![0.0; 5]);
    assert_eq!(report.contour_band, vec![1200.0; 5]);
    assert!((report.hop_ms - 10.0).abs() < 1e-6);
    assert_eq!(report.start_frame_idx, 0);
    assert_eq!(report.end_frame_idx, 5);
    assert!((report.total_duration - 50.0).abs() < 1e-6);
}

#[test]
#[should_panic]
fn panics_when_reference_slice_is_out_of_range() {
    let reference = reference_features(vec![0.1, 0.2], vec![200.0, 210.0]);
    let learner = chunk_features(vec![0.1], vec![200.0]);
    let _ = align_features(&reference, &learner, 2, 0.0, SAMPLE_RATE, HOP_SAMPLES);
}

#[test]
#[should_panic]
fn panics_on_pitch_length_mismatch() {
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
    let _ = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
}

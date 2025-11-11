use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use approx::assert_abs_diff_eq;
use flowalyzer::pronunciation::features::FeatureExtractor;
use flowalyzer::pronunciation::{PronunciationFeatures, RecordedClip};
use hound::WavReader;
use ndarray::{Array1, Array2, Axis};
use serde::Deserialize;

const FIXTURE_DIR: &str = "tests/fixtures/features";

#[derive(Deserialize)]
struct ExpectedFeatures {
    frame_count: usize,
    mel_bands: usize,
    mel: Vec<Vec<f32>>,
    spectral_flux: Vec<f32>,
    energy: Vec<f32>,
    mfcc: Vec<Vec<f32>>,
    deltas: Vec<Vec<f32>>,
    delta_deltas: Vec<Vec<f32>>,
    pitch_contour: Vec<f32>,
}

#[test]
fn reference_features_match_expected_fixture() {
    let clip = load_clip("reference.wav");
    let expected = load_expected("reference_expected.json");

    let features = FeatureExtractor::new()
        .extract(&clip)
        .expect("feature extraction succeeds");

    assert_features_match(&features, &expected);
}

#[test]
fn learner_features_align_with_reference_after_normalization() {
    let reference_clip = load_clip("reference.wav");
    let learner_clip = load_clip("learner.wav");

    let extractor = FeatureExtractor::new();
    let reference = extractor
        .extract(&reference_clip)
        .expect("reference feature extraction");
    let learner = extractor
        .extract(&learner_clip)
        .expect("learner feature extraction");

    assert_arrays_close(reference.mel_spectrogram, learner.mel_spectrogram, 1e-5);
    assert_arrays_close(reference.mfcc, learner.mfcc, 1e-5);
    assert_arrays_close(reference.deltas, learner.deltas, 1e-5);
    assert_arrays_close(reference.delta_deltas, learner.delta_deltas, 1e-5);
    assert_vectors_close(reference.spectral_flux, learner.spectral_flux, 1e-5);
    assert_vectors_close(reference.energy, learner.energy, 1e-5);
    assert_vectors_close(reference.pitch_contour, learner.pitch_contour, 1e-5);
}

fn load_clip(filename: &str) -> RecordedClip {
    let path = Path::new(FIXTURE_DIR).join(filename);
    let mut reader =
        WavReader::open(&path).unwrap_or_else(|err| panic!("failed to open {:?}: {}", path, err));
    let spec = reader.spec();
    assert_eq!(
        spec.channels, 1,
        "fixtures expected to be mono for pronunciation pipeline"
    );

    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.expect("valid sample") as f32 / i16::MAX as f32)
        .collect();
    let duration_secs = samples.len() as f64 / spec.sample_rate as f64;

    RecordedClip {
        samples: Arc::from(samples),
        sample_rate: spec.sample_rate,
        channels: spec.channels as u8,
        duration: Duration::from_secs_f64(duration_secs),
    }
}

fn load_expected(filename: &str) -> ExpectedFeatures {
    let path = Path::new(FIXTURE_DIR).join(filename);
    let reader =
        File::open(&path).unwrap_or_else(|err| panic!("failed to open {:?}: {}", path, err));
    serde_json::from_reader(reader)
        .unwrap_or_else(|err| panic!("failed to deserialize {:?}: {}", path, err))
}

fn assert_features_match(features: &PronunciationFeatures, expected: &ExpectedFeatures) {
    assert_eq!(features.frame_count, expected.frame_count);
    assert_eq!(features.mel_bands, expected.mel_bands);

    assert_matrix_eq(&features.mel_spectrogram, &expected.mel, 1e-6);
    assert_vector_eq(&features.spectral_flux, &expected.spectral_flux, 1e-6);
    assert_vector_eq(&features.energy, &expected.energy, 1e-6);
    assert_matrix_eq(&features.mfcc, &expected.mfcc, 1e-6);
    assert_matrix_eq(&features.deltas, &expected.deltas, 1e-6);
    assert_matrix_eq(&features.delta_deltas, &expected.delta_deltas, 1e-6);
    assert_vector_eq(&features.pitch_contour, &expected.pitch_contour, 1e-6);
}

fn assert_matrix_eq(matrix: &Array2<f32>, expected: &[Vec<f32>], tol: f32) {
    assert_eq!(
        matrix.len_of(Axis(0)),
        expected.len(),
        "matrix row count mismatch"
    );
    if let Some(row) = expected.first() {
        assert_eq!(
            matrix.len_of(Axis(1)),
            row.len(),
            "matrix column count mismatch"
        );
    }
    for (row_idx, row) in matrix.outer_iter().enumerate() {
        for (col_idx, value) in row.iter().enumerate() {
            let expected_value = expected[row_idx][col_idx];
            assert_abs_diff_eq!(*value, expected_value, epsilon = tol);
        }
    }
}

fn assert_vector_eq(vector: &Array1<f32>, expected: &[f32], tol: f32) {
    assert_eq!(vector.len(), expected.len(), "vector length mismatch");
    for (value, expected_value) in vector.iter().zip(expected.iter()) {
        assert_abs_diff_eq!(*value, *expected_value, epsilon = tol);
    }
}

fn assert_arrays_close(lhs: Array2<f32>, rhs: Array2<f32>, tol: f32) {
    assert_eq!(lhs.dim(), rhs.dim(), "array shapes differ");
    for (a, b) in lhs.iter().zip(rhs.iter()) {
        assert_abs_diff_eq!(*a, *b, epsilon = tol);
    }
}

fn assert_vectors_close(lhs: Array1<f32>, rhs: Array1<f32>, tol: f32) {
    assert_eq!(lhs.len(), rhs.len(), "vector lengths differ");
    for (a, b) in lhs.iter().zip(rhs.iter()) {
        assert_abs_diff_eq!(*a, *b, epsilon = tol);
    }
}

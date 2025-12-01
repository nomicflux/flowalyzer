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

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().copied().sum::<f32>() / values.len() as f32
}

#[test]
fn given_silence_reference_and_silence_learner_then_similarity_is_max() {
    // Given both reference and learner are silent
    let reference = reference_features(vec![0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0]);
    let learner = chunk_features(vec![0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0]);

    // When we align them
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    // Then similarity is at the top end and contour stays silent
    assert!(report.similarity_band.iter().all(|v| *v > 0.95));
    assert!(report.contour_band.iter().all(|v| v.abs() < 1e-6));
}

#[test]
fn given_silence_reference_and_voiced_learner_then_similarity_is_worst() {
    // Given the reference is silent and the learner is voiced
    let reference = reference_features(vec![0.0, 0.0], vec![0.0, 0.0]);
    let learner = chunk_features(vec![1.0, 1.0], vec![120.0, 140.0]);

    // When we align them
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    // Then similarity is driven to the worst end (negative) and contour stays zero because reference is unvoiced
    assert!(report.similarity_band.iter().all(|v| *v < 0.0));
    assert!(report.contour_band.iter().all(|v| *v == 0.0));
}

#[test]
fn given_voiced_reference_and_voiced_learner_with_match_then_similarity_near_top() {
    // Given matching voiced reference and learner frames
    let reference = reference_features(vec![1.2, 1.0], vec![200.0, 210.0]);
    let learner = chunk_features(vec![1.2, 1.0], vec![200.0, 210.0]);

    // When we align them
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    // Then similarity stays near the top and contour differences stay near zero
    assert!(report.similarity_band.iter().all(|v| *v > 0.95));
    assert!(report.contour_band.iter().all(|v| v.abs() < 1e-6));
}

#[test]
fn given_voiced_reference_and_voiced_learner_with_mismatch_then_similarity_degrades() {
    // Given mismatched energy and pitch between reference and learner
    let reference = reference_features(vec![1.5, 1.5], vec![180.0, 200.0]);
    let learner = chunk_features(vec![0.5, 2.0], vec![150.0, 260.0]);

    // When we align them
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    // Then similarity degrades (falls below a near-top threshold) but remains finite and not clamped away
    assert!(report.similarity_band.iter().any(|v| *v < 0.8));
    assert!(report.similarity_band.iter().all(|v| v.is_finite()));
}

#[test]
fn given_reference_silence_against_speech_then_similarity_persists_negative_values() {
    // Given reference silence and strong learner speech
    let reference = reference_features(vec![0.0, 0.0], vec![0.0, 0.0]);
    let learner = chunk_features(vec![10.0, 10.0], vec![400.0, 380.0]);

    // When we align them
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    // Then similarity includes negative values (preserved instead of clamped) indicating worst-case mismatch
    assert!(report.similarity_band.iter().all(|v| *v < 0.0));
}

#[test]
fn given_session_scale_strong_mismatch_becomes_more_negative_than_moderate() {
    // Given a reference with varying energy and pitch
    let reference = reference_features(vec![1.0, 2.0, 1.5], vec![200.0, 210.0, 220.0]);

    // And three learner variants: perfect match, moderate mismatch, and strong mismatch
    let perfect = chunk_features(vec![1.0, 2.0, 1.5], vec![200.0, 210.0, 220.0]);
    let moderate = chunk_features(vec![2.0, 3.0, 1.5], vec![260.0, 250.0, 240.0]);
    let strong = chunk_features(vec![8.0, 8.0, 8.0], vec![400.0, 380.0, 360.0]);

    // When we align them
    let perfect_report = align_features(&reference, &perfect, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    let moderate_report = align_features(&reference, &moderate, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);
    let strong_report = align_features(&reference, &strong, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    let perfect_mean = mean(&perfect_report.similarity_band);
    let moderate_mean = mean(&moderate_report.similarity_band);
    let strong_mean = mean(&strong_report.similarity_band);

    // Then perfect > moderate > strong, and strong falls below zero
    assert!(perfect_mean > moderate_mean, "perfect should outrank moderate");
    assert!(moderate_mean > strong_mean, "moderate should outrank strong");
    assert!(strong_mean < 0.0, "strong mismatch should become negative, not clamped");
    assert!(perfect_mean > 0.9, "perfect match should stay near the top of the scale");
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

#[test]
fn voiced_silence_voiced_contour_tracks_voiced_and_zeros_silence() {
    let reference = reference_features(
        vec![1.0; 6],
        vec![200.0, 210.0, 0.0, 0.0, 230.0, 240.0],
    );
    let learner = chunk_features(
        vec![1.0; 6],
        vec![198.0, 212.0, 0.0, 0.0, 228.0, 242.0],
    );
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    assert_eq!(
        report.contour_band[2], 0.0,
        "silence frame should produce zero contour"
    );
    assert_eq!(
        report.contour_band[3], 0.0,
        "silence frame should produce zero contour"
    );

    let voiced = &[report.contour_band[0], report.contour_band[1], report.contour_band[4], report.contour_band[5]];
    assert!(
        voiced.iter().all(|v| v.abs() < 0.05),
        "voiced frames should be close after offset normalization: {:?}",
        voiced
    );
}

#[test]
fn smoothing_preserves_voiced_shape_and_zeroes_unvoiced() {
    let reference = reference_features(
        vec![1.0; 7],
        vec![200.0, 205.0, 0.0, 0.0, 210.0, 215.0, 220.0],
    );
    let learner = chunk_features(
        vec![1.0; 7],
        vec![202.0, 207.0, 0.0, 0.0, 208.0, 213.0, 218.0],
    );
    let report = align_features(&reference, &learner, 0, 0.0, SAMPLE_RATE, HOP_SAMPLES);

    // Expect unvoiced frames smoothed to zero
    assert_eq!(report.contour_band[2], 0.0);
    assert_eq!(report.contour_band[3], 0.0);

    // Expect voiced frames to retain the general contour (small deltas around 0)
    let voiced = &[report.contour_band[0], report.contour_band[1], report.contour_band[4], report.contour_band[5], report.contour_band[6]];
    assert!(
        voiced.iter().all(|v| v.abs() < 0.08),
        "voiced frames should stay close after smoothing: {:?}",
        voiced
    );
}

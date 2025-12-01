use std::f32::consts::PI;

use flowalyzer::pronunciation::features::{FeatureConfig, FeatureExtractor};

#[test]
fn reference_frame_count_matches_expected_math() {
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let total_len = cfg.frame_len_samples + cfg.hop_samples * 4 + cfg.hop_samples / 2;
    let samples: Vec<f32> = (0..total_len)
        .map(|i| (2.0 * PI * 100.0 * i as f32 / sample_rate as f32).sin())
        .collect();

    let extractor = FeatureExtractor::new();
    let features = extractor.extract_reference(&samples, sample_rate, cfg);
    let expected_frames = ((total_len - cfg.frame_len_samples) / cfg.hop_samples) + 1;

    assert_eq!(features.energy.len(), expected_frames);
    assert_eq!(features.pitch.len(), expected_frames);
    assert_eq!(features.frame_starts.len(), expected_frames);
}

#[test]
fn repeated_extractions_are_deterministic() {
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let samples: Vec<f32> = (0..(cfg.frame_len_samples * 2))
        .map(|i| (2.0 * PI * 180.0 * i as f32 / sample_rate as f32).sin())
        .collect();
    let extractor = FeatureExtractor::new();

    let first = extractor.extract_reference(&samples, sample_rate, cfg);
    let second = extractor.extract_reference(&samples, sample_rate, cfg);

    assert_eq!(first.energy, second.energy);
    assert_eq!(first.pitch, second.pitch);
    assert_eq!(first.frame_starts, second.frame_starts);
}

#[test]
#[should_panic]
fn reference_extraction_panics_when_too_short() {
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let extractor = FeatureExtractor::new();
    let samples = vec![0.0; cfg.frame_len_samples - 1];

    extractor.extract_reference(&samples, sample_rate, cfg);
}

#[test]
fn chunk_extraction_returns_empty_when_window_too_short() {
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let extractor = FeatureExtractor::new();
    let tail = vec![0.0; cfg.frame_len_samples - cfg.hop_samples];
    let chunk = vec![0.0; cfg.hop_samples - 1];

    let features = extractor.extract_chunk(&tail, &chunk, sample_rate, cfg, 0);
    assert!(features.energy.is_empty());
    assert!(features.pitch.is_empty());
    assert!(features.frame_starts.is_empty());
}

#[test]
fn chunk_preserves_hop_phase_with_tail_contribution() {
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let tail_len = cfg.frame_len_samples - cfg.hop_samples;
    let prev_tail: Vec<f32> = (0..tail_len)
        .map(|i| (2.0 * PI * 220.0 * i as f32 / sample_rate as f32).sin())
        .collect();
    let chunk_len = cfg.hop_samples * 7;
    let chunk: Vec<f32> = vec![0.0; chunk_len];

    let extractor = FeatureExtractor::new();
    let features = extractor.extract_chunk(&prev_tail, &chunk, sample_rate, cfg, 0);

    assert_eq!(features.frame_starts.first().copied(), Some(-704));
    assert!(features
        .frame_starts
        .windows(2)
        .all(|window| window[1] - window[0] == cfg.hop_samples as isize));
    assert!(
        features.energy.first().copied().unwrap_or(0.0) > 0.0,
        "tail energy should contribute to first chunk frame"
    );
}

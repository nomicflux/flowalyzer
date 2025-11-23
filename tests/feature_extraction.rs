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
fn chunk_frames_anchor_to_chunk_start() {
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let tail_len = cfg.frame_len_samples - cfg.hop_samples;
    let prev_tail: Vec<f32> = (0..tail_len)
        .map(|i| (2.0 * PI * 220.0 * i as f32 / sample_rate as f32).sin())
        .collect();
    let chunk_len = cfg.frame_len_samples + cfg.hop_samples;
    let chunk: Vec<f32> = (0..chunk_len)
        .map(|i| (2.0 * PI * 330.0 * i as f32 / sample_rate as f32).sin())
        .collect();

    let extractor = FeatureExtractor::new();
    let features = extractor.extract_chunk(&prev_tail, &chunk, sample_rate, cfg);

    assert_eq!(features.frame_starts, vec![0, cfg.hop_samples]);
    assert_eq!(features.energy.len(), features.frame_starts.len());
    assert_eq!(features.pitch.len(), features.frame_starts.len());
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
fn chunk_extraction_panics_on_invalid_inputs() {
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let extractor = FeatureExtractor::new();
    let chunk = vec![0.0; cfg.frame_len_samples];
    let wrong_tail = vec![0.0; 1];

    assert!(std::panic::catch_unwind(|| {
        extractor.extract_reference(&chunk, 0, cfg);
    })
    .is_err());

    assert!(std::panic::catch_unwind(|| {
        extractor.extract_chunk(&wrong_tail, &chunk, sample_rate, cfg);
    })
    .is_err());

    assert!(std::panic::catch_unwind(|| {
        extractor.extract_chunk(
            &vec![0.0; cfg.frame_len_samples - cfg.hop_samples],
            &[],
            sample_rate,
            cfg,
        );
    })
    .is_err());
}

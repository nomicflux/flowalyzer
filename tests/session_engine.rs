use flowalyzer::pronunciation::features::FeatureConfig;
use flowalyzer::pronunciation::session::{SessionConfig, SessionEngine};

fn reference_signal(len: usize, sample_rate: u32) -> Vec<f32> {
    (0..len)
        .map(|i| (2.0 * std::f32::consts::PI * 220.0 * i as f32 / sample_rate as f32).sin())
        .collect()
}

fn seed_engine_tail(engine: &mut SessionEngine, reference: &[f32], feature_cfg: FeatureConfig) {
    let tail_len = feature_cfg.frame_len_samples - feature_cfg.hop_samples;
    let tail = reference[..tail_len].to_vec();
    engine.seed_tail(&tail);
}

#[test]
fn engine_advances_global_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let chunk = vec![0.0; (config.sample_rate * config.chunk_duration_ms / 1_000) as usize];
    let report_one = engine.process_chunk(&chunk);
    let expected_offset =
        (feature_cfg.frame_len_samples - feature_cfg.hop_samples) as f32 / config.sample_rate as f32
            * 1000.0;
    assert!((report_one.global_time_offset_ms - expected_offset).abs() < 1e-3);
    let report_two = engine.process_chunk(&chunk);
    assert!(report_two.global_time_offset_ms > report_one.global_time_offset_ms);
}

#[test]
fn engine_reset_clears_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let chunk = vec![0.0; (config.sample_rate * config.chunk_duration_ms / 1_000) as usize];
    let _ = engine.process_chunk(&chunk);
    engine.reset();
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let report_after_reset = engine.process_chunk(&chunk);
    let expected_offset =
        (feature_cfg.frame_len_samples - feature_cfg.hop_samples) as f32 / config.sample_rate as f32
            * 1000.0;
    assert!((report_after_reset.global_time_offset_ms - expected_offset).abs() < 1e-3);
}

#[test]
fn processing_multiple_chunks_updates_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let chunk = vec![0.0; (config.sample_rate * config.chunk_duration_ms / 1_000) as usize];
    let first = engine.process_chunk(&chunk);
    let second = engine.process_chunk(&chunk);
    let third = engine.process_chunk(&chunk);
    assert!(second.global_time_offset_ms > first.global_time_offset_ms);
    assert!(third.global_time_offset_ms > second.global_time_offset_ms);
}

#[test]
fn first_chunk_produces_feature_frames() {
    let config = SessionConfig {
        chunk_duration_ms: 200,
        ..Default::default()
    };
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let chunk_len = (config.sample_rate as usize * config.chunk_duration_ms as usize) / 1_000;
    let chunk = reference[feature_cfg.frame_len_samples - feature_cfg.hop_samples
        ..feature_cfg.frame_len_samples - feature_cfg.hop_samples + chunk_len]
        .to_vec();
    let report = engine.process_chunk(&chunk);
    assert!(
        !report.similarity_band.is_empty(),
        "similarity should contain frames on first chunk"
    );
    assert!(
        !report.contour_band.is_empty(),
        "contour should contain frames on first chunk"
    );
}

#[test]
fn trimming_respects_pitch_length() {
    let config = SessionConfig {
        chunk_duration_ms: 150,
        ..Default::default()
    };
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let chunk = vec![0.0; feature_cfg.frame_len_samples + feature_cfg.hop_samples];
    let _ = engine.process_chunk(&chunk);
    // Processing a second chunk should not panic even if learner pitch has fewer frames than energy.
    let _report = engine.process_chunk(&chunk);
}

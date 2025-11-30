use flowalyzer::pronunciation::features::{FeatureConfig, FeatureExtractor};
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

fn build_engine(reference: &[f32], config: &SessionConfig) -> (SessionEngine, FeatureConfig) {
    let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
    let extractor = FeatureExtractor::new();
    let reference_features =
        extractor.extract_reference(reference, config.sample_rate, feature_cfg);
    let engine = SessionEngine::new(reference_features, config.sample_rate);
    (engine, feature_cfg)
}

#[test]
#[should_panic]
fn process_chunk_panics_without_seed_tail() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let (mut engine, feature_cfg) = build_engine(&reference, &config);
    let chunk = vec![0.0; feature_cfg.hop_samples];
    let _ = engine.process_chunk(&chunk);
}

#[test]
#[should_panic]
fn constructing_with_short_reference_panics() {
    let config = SessionConfig::default();
    let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
    let reference = vec![0.0; feature_cfg.frame_len_samples - 1];
    let _ = build_engine(&reference, &config);
}

#[test]
fn engine_advances_global_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let (mut engine, feature_cfg) = build_engine(&reference, &config);
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let chunk = vec![0.0; (config.sample_rate * config.chunk_duration_ms / 1_000) as usize];
    let report_one = engine.process_chunk(&chunk);
    let expected_offset = (feature_cfg.frame_len_samples - feature_cfg.hop_samples) as f32
        / config.sample_rate as f32
        * 1000.0;
    assert!((report_one.global_time_offset_ms - expected_offset).abs() < 1e-3);
    let report_two = engine.process_chunk(&chunk);
    assert!(report_two.global_time_offset_ms > report_one.global_time_offset_ms);
}

#[test]
fn engine_reset_clears_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let (mut engine, feature_cfg) = build_engine(&reference, &config);
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let chunk = vec![0.0; (config.sample_rate * config.chunk_duration_ms / 1_000) as usize];
    let _ = engine.process_chunk(&chunk);
    engine.reset();
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let report_after_reset = engine.process_chunk(&chunk);
    let expected_offset = (feature_cfg.frame_len_samples - feature_cfg.hop_samples) as f32
        / config.sample_rate as f32
        * 1000.0;
    assert!((report_after_reset.global_time_offset_ms - expected_offset).abs() < 1e-3);
}

#[test]
fn processing_multiple_chunks_updates_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let (mut engine, feature_cfg) = build_engine(&reference, &config);
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
    let (mut engine, feature_cfg) = build_engine(&reference, &config);
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
    let (mut engine, feature_cfg) = build_engine(&reference, &config);
    seed_engine_tail(&mut engine, &reference, feature_cfg);
    let chunk = vec![0.0; feature_cfg.frame_len_samples + feature_cfg.hop_samples];
    let _ = engine.process_chunk(&chunk);
    // Processing a second chunk should not panic even if learner pitch has fewer frames than energy.
    let _report = engine.process_chunk(&chunk);
}

#[test]
fn non_multiple_tail_preserves_phase_and_state() {
    let config = SessionConfig {
        chunk_duration_ms: 120,
        ..Default::default()
    };
    let reference = reference_signal((config.sample_rate * 3) as usize, config.sample_rate);
    let (mut engine, feature_cfg) = build_engine(&reference, &config);
    let required_tail_len = feature_cfg.frame_len_samples - feature_cfg.hop_samples;
    let chunk_len = feature_cfg.hop_samples * 6 + 40; // non-multiple of hop for spacing check
    let tail = reference[..required_tail_len].to_vec();
    engine.seed_tail(&tail);

    let chunk = vec![0.0; chunk_len];
    let window = {
        let mut w = Vec::with_capacity(tail.len() + chunk.len());
        w.extend_from_slice(&tail);
        w.extend_from_slice(&chunk);
        w
    };
    let expected_tail = window[window.len() - required_tail_len..].to_vec();
    let expected_start = required_tail_len as u64 / feature_cfg.hop_samples as u64;
    let expected_offset_ms = (required_tail_len as f32 / config.sample_rate as f32) * 1_000.0;

    let first_report = engine.process_chunk(&chunk);
    assert_eq!(first_report.start_frame_idx, expected_start as usize);
    assert!(
        (first_report.global_time_offset_ms - expected_offset_ms).abs() < 1e-3,
        "offset should reflect seeded tail"
    );
    assert!(
        first_report.learner_energy.first().copied().unwrap_or(0.0) > 0.0,
        "first frame energy should include tail contribution"
    );
    assert_eq!(engine.tail_samples(), expected_tail.as_slice());

    let second_expected_start =
        engine.global_sample_counter() / feature_cfg.hop_samples as u64;
    let second_expected_offset_ms =
        (engine.global_sample_counter() as f32 / config.sample_rate as f32) * 1_000.0;
    let second_report = engine.process_chunk(&chunk);
    assert_eq!(
        second_report.start_frame_idx,
        second_expected_start as usize
    );
    assert!(
        (second_report.global_time_offset_ms - second_expected_offset_ms).abs() < 1e-3,
        "offset should advance with the global sample counter"
    );
    assert_eq!(
        second_report.start_frame_idx - first_report.start_frame_idx,
        chunk_len / feature_cfg.hop_samples,
        "start_frame_idx should advance by chunk_len/hop"
    );
    assert_eq!(
        engine.tail_samples().len(),
        required_tail_len,
        "tail should keep required length after processing"
    );
}

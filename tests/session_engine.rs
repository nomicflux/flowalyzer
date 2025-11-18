use flowalyzer::pronunciation::session::{SessionConfig, SessionEngine};

fn reference_signal(len: usize, sample_rate: u32) -> Vec<f32> {
    (0..len)
        .map(|i| (2.0 * std::f32::consts::PI * 220.0 * i as f32 / sample_rate as f32).sin())
        .collect()
}

#[test]
fn engine_advances_global_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let chunk = vec![0.0; (config.sample_rate * config.chunk_duration_ms / 1_000) as usize];
    let report_one = engine.process_chunk(&chunk);
    assert_eq!(report_one.global_time_offset_ms, 0.0);
    let report_two = engine.process_chunk(&chunk);
    assert!(report_two.global_time_offset_ms > report_one.global_time_offset_ms);
}

#[test]
fn engine_reset_clears_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let chunk = vec![0.0; (config.sample_rate * config.chunk_duration_ms / 1_000) as usize];
    let _ = engine.process_chunk(&chunk);
    engine.reset();
    let report_after_reset = engine.process_chunk(&chunk);
    assert_eq!(report_after_reset.global_time_offset_ms, 0.0);
}

#[test]
fn processing_multiple_chunks_updates_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let chunk = vec![0.0; (config.sample_rate * config.chunk_duration_ms / 1_000) as usize];
    let first = engine.process_chunk(&chunk);
    let second = engine.process_chunk(&chunk);
    let third = engine.process_chunk(&chunk);
    assert!(second.global_time_offset_ms > first.global_time_offset_ms);
    assert!(third.global_time_offset_ms > second.global_time_offset_ms);
}

#[test]
fn first_chunk_produces_feature_frames() {
    let mut config = SessionConfig::default();
    config.chunk_duration_ms = 200;
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let chunk_len = (config.sample_rate as usize * config.chunk_duration_ms as usize) / 1_000;
    let chunk = reference[..chunk_len].to_vec();
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

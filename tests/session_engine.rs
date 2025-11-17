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
    let chunk = vec![0.0; engine.chunk_samples()];
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
    let chunk = vec![0.0; engine.chunk_samples()];
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
    let chunk = vec![0.0; engine.chunk_samples()];
    let first = engine.process_chunk(&chunk);
    let second = engine.process_chunk(&chunk);
    let third = engine.process_chunk(&chunk);
    assert!(second.global_time_offset_ms > first.global_time_offset_ms);
    assert!(third.global_time_offset_ms > second.global_time_offset_ms);
}

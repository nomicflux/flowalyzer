use flowalyzer::pronunciation::session::{ChunkMemoryLimit, SessionConfig, SessionEngine};

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
    let report_one = engine
        .ingest_chunk(chunk.clone())
        .expect("immediate report");
    assert_eq!(report_one.global_time_offset_ms, 0.0);
    let report_two = engine.ingest_chunk(chunk.clone()).expect("second report");
    assert!(report_two.global_time_offset_ms > report_one.global_time_offset_ms);
}

#[test]
fn engine_reset_clears_offset() {
    let config = SessionConfig::default();
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let chunk = vec![0.0; engine.chunk_samples()];
    let _ = engine.ingest_chunk(chunk.clone());
    engine.reset();
    let report_after_reset = engine.ingest_chunk(chunk).expect("report after reset");
    assert_eq!(report_after_reset.global_time_offset_ms, 0.0);
}

#[test]
fn lookahead_mode_delays_first_chunk() {
    let config = SessionConfig {
        chunk_memory_limit: ChunkMemoryLimit::PreviousAndNext,
        ..SessionConfig::default()
    };
    let reference = reference_signal((config.sample_rate * 2) as usize, config.sample_rate);
    let mut engine = SessionEngine::new(&reference, &config);
    let chunk = vec![0.0; engine.chunk_samples()];
    assert!(engine.ingest_chunk(chunk.clone()).is_none());
    let second = engine
        .ingest_chunk(chunk.clone())
        .expect("second chunk produces report");
    assert!(second.global_time_offset_ms >= 0.0);
    let flushed = engine.flush_pending().expect("flush last chunk");
    assert!(flushed.global_time_offset_ms >= second.global_time_offset_ms);
}

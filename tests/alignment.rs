use flowalyzer::pronunciation::alignment::align_chunk;

#[test]
fn identical_chunks_produce_high_similarity() {
    let energy = vec![0.5, 0.6, 0.55];
    let pitch = vec![220.0, 225.0, 230.0];
    let report = align_chunk(&energy, &energy, &pitch, &pitch, 0.0);
    assert!(report.similarity_band.iter().all(|value| *value > 0.99));
    assert!(report.contour_band.iter().all(|value| *value > 0.99));
    assert!(report.confidence > 0.99);
}

#[test]
fn mismatched_chunks_reduce_confidence() {
    let reference_energy = vec![0.5, 0.6, 0.55];
    let learner_energy = vec![0.2, 0.1, 0.3];
    let reference_pitch = vec![220.0, 225.0, 230.0];
    let learner_pitch = vec![110.0, 115.0, 120.0];
    let report = align_chunk(
        &reference_energy,
        &learner_energy,
        &reference_pitch,
        &learner_pitch,
        120.0,
    );
    assert!(report.confidence < 0.8);
    assert_eq!(report.global_time_offset_ms, 120.0);
}

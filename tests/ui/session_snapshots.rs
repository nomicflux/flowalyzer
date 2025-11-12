use flowalyzer::pronunciation::{
    AlignmentReport, PronunciationScores, SessionSnapshot,
};

#[test]
fn alignment_update_carries_energy_and_pitch_series() {
    let alignment = AlignmentReport {
        reference_energy: vec![0.2, 0.4, 0.6],
        learner_energy: vec![0.1, 0.3, 0.5],
        reference_pitch: vec![0.1, -0.2, 0.2],
        learner_pitch: vec![0.0, -0.1, 0.3],
        similarity_band: vec![0.8, 0.9],
        contour_band: vec![0.7, 0.6],
        ..AlignmentReport::default()
    };
    let snapshot = SessionSnapshot::default().with_alignment(alignment, PronunciationScores::default());
    assert_eq!(snapshot.alignment.reference_energy.len(), 3);
    assert_eq!(snapshot.alignment.learner_pitch.len(), 3);
}

#[test]
fn latency_update_preserves_alignment_data() {
    let alignment = AlignmentReport {
        reference_energy: vec![0.5],
        learner_energy: vec![0.4],
        ..AlignmentReport::default()
    };
    let snapshot = SessionSnapshot::default()
        .with_alignment(alignment, PronunciationScores::default())
        .with_latency(180.0, 200);
    assert!(snapshot.error.is_none());
    assert_eq!(snapshot.alignment.reference_energy.len(), 1);
}


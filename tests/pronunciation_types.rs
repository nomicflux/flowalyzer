use flowalyzer::pronunciation::session::{AlignmentReport, SessionSnapshot};

#[test]
fn snapshot_carries_alignment() {
    let alignment = AlignmentReport {
        reference_energy: vec![1.0],
        learner_energy: vec![1.0],
        energy_error: vec![0.0],
        reference_pitch: vec![100.0],
        learner_pitch: vec![100.0],
        similarity_band: vec![0.0],
        contour_band: vec![0.0],
        start_frame_idx: 0,
        end_frame_idx: 1,
        hop_ms: 10.0,
        global_time_offset_ms: 0.0,
        total_duration: 10.0,
    };
    let snapshot = SessionSnapshot {
        alignment: alignment.clone(),
        recording: false,
        reference_playing: false,
    };
    assert_eq!(
        snapshot.alignment.reference_energy,
        alignment.reference_energy
    );
    assert!(!snapshot.recording);
    assert!(!snapshot.reference_playing);
}

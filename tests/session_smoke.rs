use anyhow::Result;
use flowalyzer::pronunciation::session::engine::{MockCapture, SessionEngine};
use flowalyzer::pronunciation::{AlignmentWeights, RecordedClip, SessionSnapshot};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 16_000;

#[test]
fn session_engine_produces_alignment_with_mock_capture() -> Result<()> {
    let reference_samples = sine_wave(440.0, 1.0);
    let learner_samples = sine_wave(445.0, 1.0);
    let reference_clip = RecordedClip::from_samples(reference_samples.clone(), SAMPLE_RATE);
    let capture = MockCapture::from_samples(SAMPLE_RATE, learner_samples, 1024);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;
    let mut snapshot = SessionSnapshot::default();
    engine.start(&mut snapshot)?;
    let mut updates = 0;
    for _ in 0..32 {
        if let Some(next) = engine.poll(&mut snapshot)? {
            assert!(next.scores.overall.is_finite());
            updates += 1;
            break;
        }
    }
    engine.stop(&mut snapshot);
    assert!(updates > 0, "engine should emit at least one update");
    Ok(())
}

#[test]
fn learner_offset_tracks_initial_silence() -> Result<()> {
    let reference_samples = sine_wave(440.0, 1.0);
    let mut learner_samples = vec![0.0; (SAMPLE_RATE as f32 * 0.1) as usize];
    learner_samples.extend_from_slice(&sine_wave(445.0, 0.5));
    let reference_clip = RecordedClip::from_samples(reference_samples, SAMPLE_RATE);
    let capture = MockCapture::from_samples(SAMPLE_RATE, learner_samples, 512);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;
    let mut snapshot = SessionSnapshot::default();
    engine.start(&mut snapshot)?;
    let mut learner_offset_ms = None;
    for _ in 0..64 {
        if let Some(update) = engine.poll(&mut snapshot)? {
            learner_offset_ms = Some(update.alignment.learner_offset_ms);
            snapshot = update;
            break;
        }
    }
    engine.stop(&mut snapshot);
    let offset = learner_offset_ms.expect("expected learner offset in alignment");
    assert!(
        offset >= 80.0 && offset <= 150.0,
        "offset {} ms outside expected window",
        offset
    );
    if let Some(first) = snapshot.alignment.phonemes.first() {
        assert!(
            (first.learner_start_ms - offset).abs() <= 60.0,
            "first learner phoneme start {} ms should reflect offset {}",
            first.learner_start_ms,
            offset
        );
    }
    assert!(
        !snapshot.alignment.similarity_band.is_empty(),
        "similarity band should contain real samples"
    );
    assert!(
        snapshot
            .alignment
            .similarity_band
            .iter()
            .any(|&value| value.is_finite() && value > 0.0 && value < 1.0),
        "similarity band should carry normalized values"
    );
    assert!(
        !snapshot.alignment.contour_band.is_empty(),
        "contour band should contain real samples"
    );
    assert!(
        snapshot
            .alignment
            .learner_pitch
            .iter()
            .any(|&pitch| pitch > 0.0 && pitch < 600.0),
        "pitch contour should remain within a realistic range"
    );
    Ok(())
}

#[test]
fn session_snapshot_propagates_errors() -> Result<()> {
    let mut snapshot = SessionSnapshot::default();
    assert!(
        snapshot.error.is_none(),
        "initial snapshot should have no error"
    );
    let error_msg = "test error message";
    snapshot = snapshot.with_error_message(error_msg.to_string());
    assert_eq!(
        snapshot.error.as_deref(),
        Some(error_msg),
        "snapshot should contain error message"
    );
    Ok(())
}

fn sine_wave(frequency: f32, duration_secs: f32) -> Vec<f32> {
    let total_samples = (SAMPLE_RATE as f32 * duration_secs) as usize;
    (0..total_samples)
        .map(|index| {
            let t = index as f32 / SAMPLE_RATE as f32;
            (2.0 * PI * frequency * t).sin()
        })
        .collect()
}

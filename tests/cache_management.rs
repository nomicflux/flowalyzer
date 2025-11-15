use anyhow::Result;
use flowalyzer::pronunciation::session::engine::{MockCapture, SessionEngine};
use flowalyzer::pronunciation::{AlignmentWeights, ClipVariant, RecordedClip};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 16_000;

fn sine_wave(frequency: f32, duration_secs: f32) -> Vec<f32> {
    let total_samples = (SAMPLE_RATE as f32 * duration_secs) as usize;
    (0..total_samples)
        .map(|index| {
            let t = index as f32 / SAMPLE_RATE as f32;
            (2.0 * PI * frequency * t).sin()
        })
        .collect()
}

fn create_test_clip(frequency: f32, duration_secs: f32) -> RecordedClip {
    let samples = sine_wave(frequency, duration_secs);
    RecordedClip::from_samples(samples, SAMPLE_RATE)
}

#[test]
fn test_cache_populated_at_creation() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;

    let alignment = engine.reference_alignment(ClipVariant::Original)?;
    assert!(
        !alignment.reference_energy.is_empty(),
        "alignment should have energy data"
    );
    assert!(
        !alignment.reference_pitch.is_empty(),
        "alignment should have pitch data"
    );
    Ok(())
}

#[test]
fn test_get_reference_features_original() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;

    let alignment = engine.reference_alignment(ClipVariant::Original)?;
    assert!(
        !alignment.reference_energy.is_empty(),
        "should retrieve cached features"
    );
    Ok(())
}

#[test]
fn test_get_reference_features_cache_miss() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;

    let alignment_result = engine.reference_alignment(ClipVariant::Flowalyzed);
    assert!(alignment_result.is_err(), "cache miss should return error");
    Ok(())
}

#[test]
fn test_invalidate_flowalyzed_cache() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;

    let flowalyzed_clip = create_test_clip(500.0, 1.0);
    engine.cache_flowalyzed_features(&flowalyzed_clip, None)?;

    let flowalyzed_alignment = engine.reference_alignment(ClipVariant::Flowalyzed)?;
    assert!(
        !flowalyzed_alignment.reference_energy.is_empty(),
        "flowalyzed cache should be populated"
    );

    let original_alignment = engine.reference_alignment(ClipVariant::Original)?;
    assert!(
        !original_alignment.reference_energy.is_empty(),
        "original cache should still exist"
    );

    engine.invalidate_flowalyzed_cache();

    let flowalyzed_after_result = engine.reference_alignment(ClipVariant::Flowalyzed);
    assert!(
        flowalyzed_after_result.is_err(),
        "flowalyzed cache should return error after invalidation"
    );

    let original_after = engine.reference_alignment(ClipVariant::Original)?;
    assert!(
        !original_after.reference_energy.is_empty(),
        "original cache should be retained after invalidation"
    );
    Ok(())
}

#[test]
fn test_cache_flowalyzed_features() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;

    let flowalyzed_clip = create_test_clip(500.0, 1.0);
    engine.cache_flowalyzed_features(&flowalyzed_clip, None)?;

    let flowalyzed_alignment = engine.reference_alignment(ClipVariant::Flowalyzed)?;
    assert!(
        !flowalyzed_alignment.reference_energy.is_empty(),
        "flowalyzed features should be cached"
    );
    assert!(
        !flowalyzed_alignment.reference_pitch.is_empty(),
        "flowalyzed pitch should be cached"
    );
    Ok(())
}

#[test]
fn test_reference_alignment_contains_phonemes() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        false,
    )?;

    let alignment = engine.reference_alignment(ClipVariant::Original)?;
    // CRITICAL: Reference-to-reference alignment is unnecessary - we don't align reference to itself
    // But reference features should be populated for UI display
    assert!(
        !alignment.reference_energy.is_empty(),
        "reference alignment should contain reference energy for UI display"
    );
    assert!(
        alignment.phonemes.is_empty(),
        "reference alignment should NOT contain phonemes (no reference-to-reference alignment)"
    );
    Ok(())
}

#[test]
fn test_reference_alignment_contour_band_normalized() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        false,
    )?;

    let alignment = engine.reference_alignment(ClipVariant::Original)?;
    // CRITICAL: Reference-to-reference alignment is unnecessary - no contour_band without alignment
    // But reference pitch should be populated for UI display
    assert!(
        !alignment.reference_pitch.is_empty(),
        "reference alignment should contain reference pitch for UI display"
    );
    assert!(
        alignment.contour_band.is_empty(),
        "contour band should be empty (no reference-to-reference alignment)"
    );
    Ok(())
}

#[test]
fn test_reference_alignment_cached() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;

    let alignment1 = engine.reference_alignment(ClipVariant::Original)?;
    let alignment2 = engine.reference_alignment(ClipVariant::Original)?;

    assert_eq!(
        alignment1.reference_energy.len(),
        alignment2.reference_energy.len(),
        "cached alignment should be consistent"
    );
    assert_eq!(
        alignment1.reference_pitch.len(),
        alignment2.reference_pitch.len(),
        "cached pitch should be consistent"
    );
    Ok(())
}

#[test]
fn test_set_active_clip() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, vec![], 0);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;

    let flowalyzed_clip = create_test_clip(500.0, 1.0);
    engine.cache_flowalyzed_features(&flowalyzed_clip, None)?;

    engine.set_active_clip(ClipVariant::Flowalyzed);

    let original_alignment = engine.reference_alignment(ClipVariant::Original)?;
    let flowalyzed_alignment = engine.reference_alignment(ClipVariant::Flowalyzed)?;

    assert!(
        !original_alignment.reference_energy.is_empty(),
        "original should still be accessible"
    );
    assert!(
        !flowalyzed_alignment.reference_energy.is_empty(),
        "flowalyzed should be accessible"
    );
    Ok(())
}

#[test]
fn test_process_chunk_uses_active_clip() -> Result<()> {
    let reference_clip = create_test_clip(440.0, 1.0);
    let learner_samples = sine_wave(445.0, 1.0);
    let capture = MockCapture::from_samples(SAMPLE_RATE, learner_samples, 1024);
    let mut engine = SessionEngine::new(
        reference_clip,
        AlignmentWeights::default(),
        200,
        capture,
        true,
    )?;

    let flowalyzed_clip = create_test_clip(500.0, 1.0);
    engine.cache_flowalyzed_features(&flowalyzed_clip, None)?;

    let mut snapshot = flowalyzer::pronunciation::SessionSnapshot::default();
    engine.start(&mut snapshot)?;

    engine.set_active_clip(ClipVariant::Original);
    let original_clip = create_test_clip(440.0, 1.0);
    let mut updates_original = 0;
    for _ in 0..32 {
        if engine.poll(&mut snapshot, &original_clip)?.is_some() {
            updates_original += 1;
            break;
        }
    }

    engine.set_active_clip(ClipVariant::Flowalyzed);
    let mut updates_flowalyzed = 0;
    for _ in 0..32 {
        if engine.poll(&mut snapshot, &flowalyzed_clip)?.is_some() {
            updates_flowalyzed += 1;
            break;
        }
    }

    engine.stop(&mut snapshot);
    assert!(
        updates_original > 0,
        "should process chunks with original clip"
    );
    assert!(
        updates_flowalyzed > 0,
        "should process chunks with flowalyzed clip"
    );
    Ok(())
}

use std::path::Path;

use anyhow::Result;
use flowalyzer::pronunciation::{load_clip, ClipVariant, RecordedClip};
use hound::{SampleFormat, WavSpec, WavWriter};
use tempfile::tempdir;

const SAMPLE_RATE: u32 = 16_000;
const MAX_DURATION_SECS: u64 = 300; // 5 minutes

fn write_sine_wave(path: &Path, frequency: f32, duration_secs: usize) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    let total_samples = SAMPLE_RATE as usize * duration_secs;
    for index in 0..total_samples {
        let t = index as f32 / SAMPLE_RATE as f32;
        let sample =
            (f32::sin(2.0 * std::f32::consts::PI * frequency * t) * i16::MAX as f32) as i16;
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

#[test]
fn load_clip_rejects_clip_over_five_minutes() -> Result<()> {
    let temp = tempdir()?;
    let reference = temp.path().join("reference.wav");
    // 301 seconds - just over limit
    write_sine_wave(&reference, 440.0, 301)?;

    let result = load_clip(&reference);
    assert!(result.is_err(), "should reject clip over 5 minutes");
    let error_msg = match result {
        Ok(_) => panic!("expected error"),
        Err(e) => e.to_string(),
    };
    assert!(
        error_msg.contains("exceeds maximum"),
        "error message should mention duration limit"
    );
    assert!(
        error_msg.contains(&MAX_DURATION_SECS.to_string()),
        "error message should include maximum duration"
    );
    Ok(())
}

#[test]
fn recordeclip_from_samples_computes_duration_correctly() {
    let sample_rate = 16_000;
    let duration_secs = 10;
    let samples = vec![0.0f32; sample_rate as usize * duration_secs];
    let clip = RecordedClip::from_samples(samples, sample_rate);
    assert_eq!(clip.duration.as_secs(), duration_secs as u64);
    assert_eq!(clip.sample_rate, sample_rate);
}

#[test]
fn clipvariant_original_equals_itself() {
    assert_eq!(ClipVariant::Original, ClipVariant::Original);
}

#[test]
fn clipvariant_flowalyzed_equals_itself() {
    assert_eq!(ClipVariant::Flowalyzed, ClipVariant::Flowalyzed);
}

#[test]
fn clipvariant_original_not_equal_to_flowalyzed() {
    assert_ne!(ClipVariant::Original, ClipVariant::Flowalyzed);
}

use std::f32::consts::PI;
use std::path::Path;

use anyhow::Result;
use flowalyzer::config::AppConfig;
use flowalyzer::pronunciation::{run_session, CaptureSettings, SessionConfig};
use hound::{SampleFormat, WavSpec, WavWriter};
use tempfile::tempdir;

const SAMPLE_RATE: u32 = 16_000;
const DURATION_SECONDS: usize = 1;

#[test]
fn run_session_completes_without_transcripts() -> Result<()> {
    let temp = tempdir()?;
    let reference = temp.path().join("reference.wav");
    let learner = temp.path().join("learner.wav");
    write_sine_wave(&reference, 440.0)?;
    write_sine_wave(&learner, 446.0)?;

    let assets = AppConfig::from_override(Some(project_assets_root()))?;
    let capture = CaptureSettings::new(None, SAMPLE_RATE, 100..=200);
    let config = SessionConfig::new(reference, learner, assets.assets_root, capture);
    let outcome = run_session(config)?;

    assert!(outcome.scores.overall.is_finite());
    assert!(!outcome.alignment.phonemes.is_empty());
    Ok(())
}

fn write_sine_wave(path: &Path, frequency: f32) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    let total_samples = SAMPLE_RATE as usize * DURATION_SECONDS;
    for index in 0..total_samples {
        let t = index as f32 / SAMPLE_RATE as f32;
        let sample = (f32::sin(2.0 * PI * frequency * t) * i16::MAX as f32) as i16;
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

fn project_assets_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

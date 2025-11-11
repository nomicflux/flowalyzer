use std::path::PathBuf;

use anyhow::Result;
use flowalyzer::config::AppConfig;
use flowalyzer::pronunciation::{
    alignment::AudioAligner, metrics::MetricCalculator, CaptureSettings, PronunciationFeatures,
    SessionConfig,
};
use flowalyzer::ui::launch_ui;
use ndarray::{Array1, Array2};

fn main() -> Result<()> {
    let reference = synthetic_features(120, 80, 13, 0.0);
    let learner = synthetic_features(120, 80, 13, 0.2);

    let aligner = AudioAligner::new();
    let alignment = aligner.align(&reference, &learner)?;

    let scores = MetricCalculator::new().score(&alignment)?;
    let assets = AppConfig::from_override(None)?;
    let capture = CaptureSettings::new(None, 16_000, 100..=200);
    let config = SessionConfig::new(
        PathBuf::from("fixtures/reference.wav"),
        PathBuf::from("fixtures/learner.wav"),
        assets.assets_root,
        capture,
    )
    .with_ui(true);

    launch_ui(&config, &alignment, &scores)?;
    Ok(())
}

fn synthetic_features(
    frame_count: usize,
    mel_bands: usize,
    mfcc_coeffs: usize,
    phase_offset: f32,
) -> PronunciationFeatures {
    let mel_spectrogram = Array2::from_shape_fn((frame_count, mel_bands), |(frame, band)| {
        let t = frame as f32 * 0.05 + band as f32 * 0.01 + phase_offset;
        (t.sin() * 0.5 + 0.5).clamp(0.0, 1.0)
    });
    let spectral_flux = Array1::from_shape_fn(frame_count, |frame| {
        ((frame as f32 * 0.03 + phase_offset).cos() + 1.0) * 0.5
    });
    let energy = Array1::from_shape_fn(frame_count, |frame| {
        ((frame as f32 * 0.02 + phase_offset).sin() + 1.2).max(0.0)
    });
    let mfcc = Array2::from_shape_fn((frame_count, mfcc_coeffs), |(frame, coeff)| {
        ((frame as f32 * 0.04 + coeff as f32 * 0.07 + phase_offset).sin()).clamp(-1.0, 1.0)
    });
    let deltas = mfcc.mapv(|value| value * 0.5);
    let delta_deltas = mfcc.mapv(|value| value * 0.25);

    PronunciationFeatures {
        frame_count,
        mel_bands,
        mel_spectrogram,
        spectral_flux,
        energy,
        mfcc,
        deltas,
        delta_deltas,
    }
}
